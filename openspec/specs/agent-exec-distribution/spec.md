### Requirement: Public release artifacts

The project SHALL produce versioned, checksum-protected command-line artifacts for each declared supported target from its designated trusted build host. GitHub Actions SHALL build Linux artifacts only. macOS artifacts SHALL be built and smoke-tested locally on `mini` before explicit upload. Windows release binaries SHALL NOT be declared or produced.

#### Scenario: Release tag creates the Linux artifact

**Given**: a valid version tag and passing repository checks
**When**: the GitHub release workflow runs
**Then**: it builds, smoke-tests, checksums, and publishes only the declared Linux x86_64 archive

#### Scenario: Local mini build creates the macOS artifact

**Given**: a valid version tag checked out on `mini`
**When**: the local macOS release command runs successfully with upload enabled
**Then**: it builds and smoke-tests the native macOS binary before uploading its versioned archive and checksum to the matching GitHub Release

#### Scenario: Unsupported target is not implied

**Given**: Windows or another target without a designated artifact path
**When**: a user reads the installation documentation
**Then**: the target is absent from the supported artifact list or explicitly marked unsupported

### Requirement: Release artifact smoke verification

Every native release executable SHALL demonstrate real `agent-exec` behavior on its designated build host before it is uploaded.

#### Scenario: Native binary passes smoke verification

**Given**: a release executable built by Linux GitHub Actions or locally on `mini` for macOS
**When**: its release path runs `agent-exec --version` and a short managed command
**Then**: the version command succeeds and the managed command returns a successful JSON response

#### Scenario: Smoke verification fails

**Given**: a release executable that cannot start or complete the short managed command
**When**: smoke verification runs
**Then**: publication for that artifact fails before the invalid artifact is uploaded

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

The README SHALL provide installation instructions that correspond to the actual Linux GitHub Actions artifact, locally built macOS artifact, and package/source fallbacks, and SHALL explicitly state that Windows release binaries are not provided.

#### Scenario: New user follows public installation path

**Given**: Linux x86_64 or a macOS platform with a published artifact and no local repository checkout
**When**: a user follows the matching README installation instructions
**Then**: the user can verify the checksum, install `agent-exec`, run `agent-exec --version`, and execute a short managed command
