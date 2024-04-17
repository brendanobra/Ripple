# GHEC Migration

There will be multiple phases to this effort

## Phase 1: Parity

### Setup Accounts and credentials to deploy the artifacts
Reuse existing service account and web hook token from the Certification team
Add the artifactory token for release once it is available.

### Setup CI CD pipeline for EOS-Ripple with existing sub module dependencies it should support
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

### Build Edge firmrware build using artifacts generated from S3

## Phase 2: Steady State

### Cleanup ripple_comcast_extns repo 

### Setup a flatter dependency structure for Eos-Ripple repo

### Setup CI CD pipeline for each Repos
 ripple-eos-distributor-extn
 ripple-eos-thunder-extn
 ripple-badger-extn


## Phase 3: Enhancement

### Create individual release pipelines for each extension

### 
