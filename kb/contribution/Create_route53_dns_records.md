# Create route53 dns records for service

## AWS command for CREATing a new DNS
```shell
aws route53 change-resource-record-sets --hosted-zone-id <hosted-zone-id>  --change-batch <change-batch-file.json>
```

The following command can be used for listing all the hosted-zone. 
```shell
aws route53 list-hosted-zones  
```

A Sample JSON file for creating a CNAME record using AWS route53 command is shown below. 
```shell
{
    "Comment": "entos-observability.svc-dev.thor.comcast.com",
    "Changes": [
        {
            "Action": "CREATE",
            "ResourceRecordSet": {
                "Name": "www.entos-observability.svc-dev.thor.comcast.com",
                "Type": "CNAME",
                "TTL": 300,
                "ResourceRecords": [
                    {
                        "Value": "d.use1.janus.comcast.com"
                    }
                ]
            }
        }
    ]
}
```

**Name** specifies the domain name for which the record is being created (eg: www.entos-observability.svc-dev.thor.comcast.com)

**ResourceRecords** Array of resource records associated with the DNS record.  For a CNAME record, there's only one resource record, which is the canonical name (CNAME).