/**
 * Test driver for the OpenCode auto-resume plugin.
 *
 *   node drive-plugin.mjs <plugin-path> <case-json-path>
 *
 * The case JSON supplies `{ context, input, output }`: the plugin factory context
 * OpenCode would pass, and the two arguments of the `tool.execute.after` hook.
 * Environment configuration is inherited from this process, so the Rust test can
 * drive every configuration branch without a live OpenCode server.
 *
 * The driver asserts the plugin's OpenCode-facing contract: exactly one export,
 * and a hook that resolves rather than throwing.
 */

import { readFileSync } from "node:fs"
import { pathToFileURL } from "node:url"

const [pluginPath, casePath] = process.argv.slice(2)
if (!pluginPath || !casePath) {
  process.stderr.write("usage: drive-plugin.mjs <plugin-path> <case-json-path>\n")
  process.exit(2)
}

const testCase = JSON.parse(readFileSync(casePath, "utf8"))
const module = await import(pathToFileURL(pluginPath).href)

const exported = Object.entries(module)
if (exported.length !== 1) {
  process.stderr.write(
    `plugin must export exactly one plugin factory, found: ${exported.map(([n]) => n).join(", ")}\n`,
  )
  process.exit(2)
}

const [, factory] = exported[0]
const hooks = await factory(testCase.context ?? {})
const hook = hooks?.["tool.execute.after"]
if (typeof hook !== "function") {
  process.stderr.write("plugin did not register a tool.execute.after hook\n")
  process.exit(2)
}

await hook(testCase.input, testCase.output)
