#!/bin/sh
fly -t ottx sp  \
  --pipeline dpab_core \
  --config ./pipeline-dpab-core.yaml \
  --load-vars-from ./params-dpab-core.yaml
