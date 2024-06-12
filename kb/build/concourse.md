# Concourse tricks
Github releases sometimes do not get picked up by the concourse job, due to issues with resource caching - the symptom will be a release being created 
in GithHub, and the concourse release job *not* starting. To address this, expire the concourse cache for the release resource with:
```
fly -t <your local target name> clear-resource-cache  -r eos-ripple/eos-ripple-final-releases

fly -t <your local target name> clear-resource-cache  -r eos-ripple/eos-ripple-pre-releases
```
