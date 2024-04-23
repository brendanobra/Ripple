# GHEC Migration

There will be multiple phases to this effort and tracked under https://ccp.sys.comcast.net/browse/RPPL-1435

## Phase 1: Parity

### Setup Accounts and credentials to deploy the artifacts
Reuse existing service account and web hook token from the Certification team
Add the artifactory token for release once it is available.

### Setup CI CD pipeline for EOS-Ripple with existing sub module dependencies it should support
https://ccp.sys.comcast.net/browse/RPPL-1985
#### PR pipeline
1. Checking Format
2. Run unit tests
3. Run Contract Tests
4. Deploy artifacts to S3

#### Release pipeline
1. Checking Format
2. Run unit tests
3. Run Contract Tests
4. Deploy artifacts to Partner Artifactory

### Build Edge firmware build using artifacts generated from S3

## Phase 2: Steady State

### Create patches for each repo from the main branch of GHE
https://ccp.sys.comcast.net/browse/RPPL-2029

### Cleanup ripple_comcast_extns repo https://ccp.sys.comcast.net/browse/RPPL-1979

Objective of this effort is to setup a flatter dependency structure for Eos-Ripple repo

### Setup CI CD pipeline for each Repos
 1. ripple-eos-distributor-extn https://ccp.sys.comcast.net/browse/RPPL-1981
 2. ripple-eos-thunder-extn https://ccp.sys.comcast.net/browse/RPPL-1989
 3. ripple-badger-extn https://ccp.sys.comcast.net/browse/RPPL-1991


## Phase 3: Enhancement

### Create individual release pipelines for each extension
https://ccp.sys.comcast.net/browse/RPPL-1987

### Try leveraging GitHub Actions more compared to Concourse < Needs Discussion and tickets>
Moving below items to Actions
1. Checking Format
2. Run Clippy
3. Run Unit Tests
4. Run Contract tests
5. Run Mock Sanity
