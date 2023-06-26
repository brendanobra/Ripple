#!/bin/sh
grpcurl -v  -H "Authorization: Bearer $SAT_TOKEN" -H "DeviceId: $DEVICE_ID" -H "AccountId: $ACCOUNT_ID" -d \
'{}' \
 ad-platform-service.svc-qa.thor.comcast.com:443 ottx.adplatform.AdPlatformService.AdInitObjectRequest

