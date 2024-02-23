# Janus
Janus is a managed kubernetes platform.



# Tools required
To interactive with Janus (kubernetes), several tools must be installed:
- [aadawscli](https://github.com/cloud-cre/AWSAzureADCLI) - Tool that exchanges comcast credentials for aws credentials

- kubectl (see your os install, on mac `brew install kubectl`)

- [aws cli](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) the aws cli


# Onboarding
ask bobra200 for onboarding

# Using
login with aadawscli:

`aadawscli`

you will be given an MFA code and a url to click - click the url as enter the code, and login.

It will look similar to this:

```
aadawscli
To sign in, use a web browser to open the page https://microsoft.com/devicelogin and enter the code BQD6VT77S to authenticate.
+----+-------------------------------+-------------------------------+--------------------------------------------------------------+
| #  |            ACCOUNT            |             ROLE              |                          ASSUME ARN                          |
+----+-------------------------------+-------------------------------+--------------------------------------------------------------+
| 1  | ottx-platform (318438517054)  | ApplicationOwner              | arn:aws:iam::318438517054:role/ApplicationOwner              |
| 2  | ottx-platform (318438517054)  | Governance/AccountManager     | arn:aws:iam::318438517054:role/Governance/AccountManager     |
| 3  | astrosam (879945319661)       | ApplicationOwner              | arn:aws:iam::879945319661:role/ApplicationOwner              |
| 4  | astrosam (879945319661)       | Governance/AccountManager     | arn:aws:iam::879945319661:role/Governance/AccountManager     |
| 5  | astrosam (879945319661)       | OneCloud/Racers_DynamoDBAdmin | arn:aws:iam::879945319661:role/OneCloud/Racers_DynamoDBAdmin |
| 6  | astrosam (879945319661)       | OneCloud/ottp-team-developer  | arn:aws:iam::879945319661:role/OneCloud/ottp-team-developer  |
| 7  | astrosam (879945319661)       | OneCloud/platco-dev           | arn:aws:iam::879945319661:role/OneCloud/platco-dev           |
| 8  | tpx-janus (991476798176)      | ApplicationOwner              | arn:aws:iam::991476798176:role/ApplicationOwner              |
| 9  | tpx-janus (991476798176)      | Governance/AccountManager     | arn:aws:iam::991476798176:role/Governance/AccountManager     |
| 10 | tpx-janus (991476798176)      | OneCloud/janus-k8s-admin      | arn:aws:iam::991476798176:role/OneCloud/janus-k8s-admin      |
| 11 | tpx-janus (991476798176)      | OneCloud/janus-k8s-user       | arn:aws:iam::991476798176:role/OneCloud/janus-k8s-user       |
| 12 | tpx-janus-test (550172920359) | ApplicationOwner              | arn:aws:iam::550172920359:role/ApplicationOwner              |
| 13 | tpx-janus-test (550172920359) | Governance/AccountManager     | arn:aws:iam::550172920359:role/Governance/AccountManager     |
| 14 | tvx-xre (540274969485)        | ApplicationOwner              | arn:aws:iam::540274969485:role/ApplicationOwner              |
+----+-------------------------------+-------------------------------+--------------------------------------------------------------+
? Pick a role from above list: 
```

Select the role: `OneCloud/janus-k8s-user`

then , setup your k8s config with:

```
aws --profile saml eks update-kubeconfig  --region us-east-1 --name <cluster name>
```
Where `<clustername>` is:

dev east a :  eks-janus-test-east1-eksCluster-3127f11
dev east b:  eks-janus-test-east1-b-eksCluster-767dd6d

For example, to login to the dev east A cluster:
```
aws --profile saml eks update-kubeconfig  --region us-east-1 --name  eks-janus-test-east1-eksCluster-3127f11

```

# Actually seeing *metrics*

Assuming you have logged in, the quickest/dirtiest way to see metrics as they are repsented in k8s is to port forward the prometheus export endpoint/port

to do this (assuming you have logged in to the development cluster), run this from the command line

```
kubectl -n entos-ripple-dev port-forward svc/otel-collector-svc 9090
```
and point a browser/cUrl at :
```http://localhost:9090/metrics```
which will show the curent export of all time series in a simple format, similar to this:
```
# HELP ripple_accessibility_voiceGuidance_milliseconds 
# TYPE ripple_accessibility_voiceGuidance_milliseconds histogram
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="0"} 0
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="5"} 0
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="10"} 0
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="25"} 1
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="50"} 1
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="75"} 1
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="100"} 1
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="250"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="500"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="750"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="1000"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="2500"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="5000"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="7500"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="10000"} 2
ripple_accessibility_voiceGuidance_milliseconds_bucket{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui",le="+Inf"} 2
ripple_accessibility_voiceGuidance_milliseconds_sum{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui"} 175
ripple_accessibility_voiceGuidance_milliseconds_count{app_id="refui",job="ripple",status="0",transport="bridge_FireboltMainApp-refui"} 2
# HELP ripple_account_session_milliseconds 
# TYPE ripple_account_session_milliseconds histogram
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="0"} 0
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="5"} 0
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="10"} 0
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="25"} 0
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="50"} 0
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="75"} 2
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="100"} 2
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="250"} 2
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="500"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="750"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="1000"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="2500"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="5000"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="7500"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="10000"} 3
ripple_account_session_milliseconds_bucket{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root",le="+Inf"} 3
ripple_account_session_milliseconds_sum{app_id="root",job="ripple",status="0",transport="bridge_FireboltMainApp-root"} 599
...
```






# Tips
janus has a development cluster, which is a safe space to do scary things (but be nice, others play there as well)

The export from ripple is curently every minute, so it requires some patience to see values updated

janus support slack channel: #janus-k8s-support