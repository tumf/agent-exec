## MODIFIED Requirements

### Requirement: Explicit registry publication

Every successfully merged Conflux Change SHALL create exactly one automatic patch-version commit. When multiple Changes are merged in one Conflux session, each Change SHALL receive its own sequential patch-version increment. Automatic versioning SHALL NOT create a release Git tag, push commits or tags, upload a GitHub Release, or publish to crates.io. Release Git tags, pushes of release history, GitHub Release uploads, and publishing to crates.io SHALL require a separate explicit release action and SHALL NOT occur as a side effect of an ordinary Conflux merge, ordinary push, or pull request.

#### Scenario: Conflux merges one Change

**Given**: Conflux accepts and merges one implementation Change
**When**: the repository `on_merged` hook runs for that Change
**Then**: the package patch version is incremented exactly once
**And**: the version update is committed
**And**: no Git tag is created
**And**: no commit or tag is pushed
**And**: no registry or GitHub Release publication is attempted

#### Scenario: Conflux merges multiple Changes

**Given**: one Conflux session processes two accepted Changes
**When**: each Change reaches its own merge completion callback
**Then**: the first callback creates one patch-version commit
**And**: the second callback creates a second sequential patch-version commit
**And**: the increments are not batched or deduplicated into one version

#### Scenario: Automatic version bump fails

**Given**: a merged Change whose version-only bump command cannot complete
**When**: the `on_merged` hook runs
**Then**: the hook reports failure
**And**: it does not create a tag, push, or publish a partial release

#### Scenario: Ordinary CI cannot publish

**Given**: a push or pull request without the explicit publication trigger
**When**: CI runs
**Then**: no crates.io publish operation is attempted

#### Scenario: Publication credentials are unavailable

**Given**: an explicit publication run without valid registry credentials
**When**: publication is requested
**Then**: the operation fails without exposing secrets or publishing a partial release
