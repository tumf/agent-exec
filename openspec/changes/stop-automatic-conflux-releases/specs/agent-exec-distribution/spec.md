## MODIFIED Requirements

### Requirement: Explicit registry publication

Version bump commits, release Git tags, pushes of release history, GitHub Release uploads, and publishing to crates.io SHALL require an explicit release action and SHALL NOT occur as a side effect of an ordinary Conflux merge, ordinary push, or pull request.

#### Scenario: Ordinary Conflux merge cannot release

**Given**: Conflux accepts and merges an implementation change
**When**: the repository `on_merged` hook runs
**Then**: it does not change the package version
**And**: it does not create a commit or Git tag
**And**: it does not push, publish, or upload release artifacts

#### Scenario: Ordinary CI cannot publish

**Given**: a push or pull request without the explicit publication trigger
**When**: CI runs
**Then**: no crates.io publish operation is attempted

#### Scenario: Publication credentials are unavailable

**Given**: an explicit publication run without valid registry credentials
**When**: publication is requested
**Then**: the operation fails without exposing secrets or publishing a partial release
