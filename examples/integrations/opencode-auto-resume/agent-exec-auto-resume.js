/**
 * OpenCode global plugin: auto-resume the originating session when an
 * agent-exec managed job finishes.
 *
 * Lifecycle
 *   1. OpenCode calls the agent-exec MCP `run` tool.
 *   2. This plugin observes `tool.execute.after`.
 *   3. For a recognized, successful, still-running job it calls
 *      `agent-exec notify set <job-id> --command "<helper> <server-url> <session-id>"`.
 *   4. The agent-exec supervisor invokes the helper at terminal state.
 *   5. The helper decides whether to resume that OpenCode session.
 *
 * This plugin never changes the MCP request, never mutates the tool result, and
 * never throws out of the hook: a failure here must not break the tool call the
 * agent is waiting on.
 *
 * Only ONE symbol is exported. OpenCode treats every module export as a plugin
 * factory, so internal helpers deliberately stay module-private.
 */

import { execFile } from "node:child_process"
import { existsSync } from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

/** agent-exec job IDs are 32 hex chars today and legacy ULIDs before that. */
const JOB_ID_PATTERN = /^[0-9A-Za-z]{8,64}$/
/** OpenCode session IDs (e.g. `ses_8f3c...`). Conservative, shell-metacharacter free. */
const SESSION_ID_PATTERN = /^[0-9A-Za-z][0-9A-Za-z._-]{0,127}$/
/** Loopback hostnames accepted as an OpenCode callback target. */
const LOOPBACK_HOSTS = new Set(["localhost", "127.0.0.1", "::1", "[::1]"])
/** Upper bound for the `agent-exec notify set` call, in milliseconds. */
const NOTIFY_TIMEOUT_MS = 15000
/** Longest tool output we attempt to scan for a run envelope. */
const MAX_OUTPUT_SCAN_BYTES = 1024 * 1024

const HELPER_FILENAME = "opencode-agent-exec-resume"

function debugLog(env, message) {
  if (env.AGENT_EXEC_OPENCODE_DEBUG) {
    process.stderr.write(`[agent-exec-auto-resume] ${message}\n`)
  }
}

/**
 * Recognize the agent-exec MCP `run` tool.
 *
 * OpenCode names MCP tools `<server-name>_<tool-name>`, and the server name is
 * whatever the user wrote in their OpenCode config (`agent-exec`, `agent_exec`,
 * `agentexec`, ...). Match on the normalized server segment instead of hardcoding
 * one spelling. `AGENT_EXEC_OPENCODE_RUN_TOOLS` overrides this with an explicit
 * comma-separated allow list.
 */
function isAgentExecRunTool(toolName, allowList) {
  if (typeof toolName !== "string" || toolName.length === 0) return false
  if (allowList.length > 0) return allowList.includes(toolName)

  const separator = Math.max(toolName.lastIndexOf("_"), toolName.lastIndexOf("."))
  if (separator <= 0) return false
  if (toolName.slice(separator + 1).toLowerCase() !== "run") return false

  const server = toolName.slice(0, separator).toLowerCase().replace(/[-_.]/g, "")
  return server === "agentexec"
}

function parseAllowList(raw) {
  if (typeof raw !== "string") return []
  return raw
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0)
}

/**
 * Find the first balanced JSON object in `text` and parse it.
 *
 * MCP clients may wrap the tool payload in surrounding text, so a plain
 * `JSON.parse` of the whole string is not enough. String literals and escapes
 * are tracked so braces inside job output cannot unbalance the scan.
 */
function firstJsonObject(text) {
  if (typeof text !== "string") return null
  const scan = text.length > MAX_OUTPUT_SCAN_BYTES ? text.slice(0, MAX_OUTPUT_SCAN_BYTES) : text

  for (let start = scan.indexOf("{"); start !== -1; start = scan.indexOf("{", start + 1)) {
    let depth = 0
    let inString = false
    let escaped = false

    for (let i = start; i < scan.length; i += 1) {
      const ch = scan[i]
      if (inString) {
        if (escaped) escaped = false
        else if (ch === "\\") escaped = true
        else if (ch === '"') inString = false
        continue
      }
      if (ch === '"') inString = true
      else if (ch === "{") depth += 1
      else if (ch === "}") {
        depth -= 1
        if (depth === 0) {
          try {
            return JSON.parse(scan.slice(start, i + 1))
          } catch {
            break
          }
        }
      }
    }
  }
  return null
}

/**
 * Validate a candidate as an agent-exec `run` success envelope for a job that is
 * still running.
 *
 * A job that already reached a terminal state inside the inline observation
 * window needs no callback: its completion event has already been dispatched,
 * and the agent has the result in hand.
 */
function runJobId(candidate) {
  if (candidate === null || typeof candidate !== "object" || Array.isArray(candidate)) return null
  if (candidate.ok !== true) return null
  if (candidate.type !== "run") return null
  if (candidate.state !== "running") return null
  if (typeof candidate.job_id !== "string" || !JOB_ID_PATTERN.test(candidate.job_id)) return null
  return candidate.job_id
}

/** Extract exactly one validated running job ID from a tool result, or null. */
function extractRunJobId(output) {
  if (output === null || typeof output !== "object") return null

  const candidates = [
    output.metadata?.structuredContent,
    output.metadata,
    typeof output.output === "string" ? firstJsonObject(output.output) : null,
  ]

  for (const candidate of candidates) {
    const jobId = runJobId(candidate)
    if (jobId !== null) return jobId
  }
  return null
}

/** Accept only plain-HTTP loopback OpenCode servers. */
function isLoopbackHttpUrl(value) {
  if (typeof value !== "string" || value.length === 0) return false
  let url
  try {
    url = new URL(value)
  } catch {
    return false
  }
  if (url.protocol !== "http:") return false
  if (url.username !== "" || url.password !== "") return false
  if (url.search !== "" || url.hash !== "") return false
  if (url.pathname !== "" && url.pathname !== "/") return false
  const hostname = url.hostname.toLowerCase()
  if (LOOPBACK_HOSTS.has(hostname)) return true
  // 127.0.0.0/8 is entirely loopback.
  return /^127\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(hostname)
}

/**
 * Resolve the OpenCode server this session is attached to.
 *
 * Explicit configuration wins so a user can pin the URL when auto-detection is
 * not available in their OpenCode build.
 */
function resolveServerUrl(ctx, env) {
  const candidates = [
    env.AGENT_EXEC_OPENCODE_SERVER_URL,
    ctx?.client?.baseUrl,
    ctx?.client?.baseURL,
    ctx?.client?.config?.baseUrl,
    ctx?.client?.config?.baseURL,
    env.OPENCODE_SERVER,
  ]
  for (const candidate of candidates) {
    if (typeof candidate === "string" && isLoopbackHttpUrl(candidate)) {
      return candidate.replace(/\/+$/, "")
    }
  }
  return null
}

/** Resolve the completion helper: explicit override, sibling file, then PATH. */
function resolveHelper(env) {
  const override = env.AGENT_EXEC_OPENCODE_RESUME_BIN
  if (typeof override === "string" && override.length > 0) return override

  try {
    const sibling = path.join(path.dirname(fileURLToPath(import.meta.url)), HELPER_FILENAME)
    if (existsSync(sibling)) return sibling
  } catch {
    // import.meta.url is not a file URL; fall through to PATH resolution.
  }
  return HELPER_FILENAME
}

/**
 * POSIX single-quote escaping.
 *
 * `notify set --command` stores a shell command string that the supervisor later
 * runs through the configured shell wrapper, so every argument must be quoted
 * even though all three components are already validated.
 */
function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`
}

function runNotifySet(binary, args, env) {
  return new Promise((resolve) => {
    execFile(
      binary,
      args,
      { env, timeout: NOTIFY_TIMEOUT_MS, windowsHide: true },
      (error) => resolve(error ?? null),
    )
  })
}

export const AgentExecAutoResume = async (ctx) => {
  const env = process.env
  const allowList = parseAllowList(env.AGENT_EXEC_OPENCODE_RUN_TOOLS)
  const agentExecBin = env.AGENT_EXEC_BIN || "agent-exec"

  return {
    "tool.execute.after": async (input, output) => {
      try {
        if (!isAgentExecRunTool(input?.tool, allowList)) return

        const jobId = extractRunJobId(output)
        if (jobId === null) {
          debugLog(env, `no running agent-exec job id in ${input?.tool} result`)
          return
        }

        const sessionId = input?.sessionID
        if (typeof sessionId !== "string" || !SESSION_ID_PATTERN.test(sessionId)) {
          debugLog(env, `rejected session id for job ${jobId}`)
          return
        }

        const serverUrl = resolveServerUrl(ctx, env)
        if (serverUrl === null) {
          debugLog(env, `no loopback OpenCode server url for job ${jobId}`)
          return
        }

        const command = [resolveHelper(env), serverUrl, sessionId].map(shellQuote).join(" ")
        const error = await runNotifySet(
          agentExecBin,
          ["notify", "set", jobId, "--command", command],
          env,
        )
        if (error !== null) {
          debugLog(env, `notify set failed for job ${jobId}: ${error.message}`)
          return
        }
        debugLog(env, `attached auto-resume callback to job ${jobId}`)
      } catch (error) {
        // A hook failure must never surface as a tool failure to the agent.
        debugLog(env, `unexpected error: ${error?.message ?? error}`)
      }
    },
  }
}
