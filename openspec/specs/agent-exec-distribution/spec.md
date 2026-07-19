### Requirement: Public release artifacts

The project SHALL produce versioned, checksum-protected command-line artifacts for each declared supported target from an explicit release trigger.

#### Scenario: Release tag creates usable artifacts

**Given**: a valid version tag and passing repository checks
**When**: the release workflow runs
**Then**: each declared target receives a consistently named archive and checksum in the corresponding GitHub Release

#### Scenario: Unsupported target is not implied

**Given**: a target without a produced and tested artifact
**When**: a user reads the installation documentation
**Then**: the target is absent from the supported list or explicitly marked unsupported

### Requirement: Release artifact smoke verification

Every native release executable SHALL demonstrate real `agent-exec` behavior before it is uploaded.

#### Scenario: Native binary passes smoke verification

**Given**: a release executable built on a native matrix runner
**When**: the workflow runs `agent-exec --version` and a short managed command
**Then**: the version command succeeds and the managed command returns a successful JSON response

#### Scenario: Smoke verification fails

**Given**: a release executable that cannot start or complete the short managed command
**When**: smoke verification runs
**Then**: publication for that release fails before the invalid artifact is uploaded

### Requirement: Installable crate package

The repository SHALL produce a crates.io-compatible package that can be installed and run without undeclared repository files.

#### Scenario: Packaged crate installs in isolation

**Given**: the crate archive produced by `cargo package`
**When**: it is installed into an isolated location
**Then**: the installed `agent-exec` passes the version and managed-command smoke checks

### Requirement: Explicit registry publication

Publishing to crates.io SHALL require an explicit release action and SHALL NOT occur on ordinary pushes or pull requests.

#### Scenario: Ordinary CI cannot publish

**Given**: a push or pull request without the explicit publication trigger
**When**: CI runs
**Then**: no crates.io publish operation is attempted

#### Scenario: Publication credentials are unavailable

**Given**: an explicit publication run without valid registry credentials
**When**: publication is requested
**Then**: the operation fails without exposing secrets or publishing a partial release

### Requirement: Public installation guidance

The README SHALL provide installation instructions that correspond to the actual public package and release artifacts.

#### Scenario: New user follows public installation path

**Given**: a supported platform and no local repository checkout
**When**: a user follows the primary README installation instructions
**Then**: the user can install `agent-exec`, run `agent-exec --version`, and execute a short managed command
