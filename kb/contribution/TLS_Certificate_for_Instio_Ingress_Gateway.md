# Setting up access to your dev environments

## Install the following tools (one time setup)

- [aadawscli](https://github.com/cloud-cre/AWSAzureADCLI#installation)
- [AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-install.html)
- [Kubernetes CLI](https://docs.aws.amazon.com/eks/latest/userguide/install-kubectl.html)

Refer - [Janus Documentation](https://etwiki.sys.comcast.net/spaces/viewspace.action?key=janus) for more details about onboarding with Janus. 

### Sample installation of aadawscli
```shell
wget https://github.com/cloud-cre/AWSAzureADCLI/releases/download/v0.0.92/aadawscli-v0.0.92-interactive-linux-amd64.zip
unzip -j aadawscli-v0.0.92-interactive-linux-amd64.zip
chmod u+x aadawscli

$ cat ~/.aadawscli/config.json 
{
    "active": "comcast",
    "config_info": {
        "comcast": {
            "tenant_id": "906aefe9-76a7-4f65-b82d-5ec20775d5aa",
            "middleware_app_id": "7870de60-ec69-4e55-988a-9e60a709fc22",
            "function_app_name": ""
        }
    },
    "flags": {
        "app-id": "c50a7162-4f70-4ca2-8914-8aba9c950afb",
        "default-session-duration": 28800,
        "fallback-session-duration": 3600,
        "show-tabular-output": true,
        "profile-name": "saml"
    }
}
```
# Authenticate with AWS account with selected IAM role

This is needed periodically - auth tokens expire every 8 hours
```
./aadawscli
```
*This will open a chromium  browser and prompt you to enter password and MFA token. After successful authentication you will get the following message*

```shell
Preparing to launch Chromium. We will redirect you to the login page shortly after Chromium launches. Please wait
   5s [====================================================================] 100%
Extracting extension:  Windows Accounts
   0s [====================================================================] 100%
Launching Chromium. Please wait.
+---+--------------+-------------------------+--------------------------------------------------------+
| # |   ACCOUNT    |          ROLE           |                       ASSUME ARN                       |
+---+--------------+-------------------------+--------------------------------------------------------+
| 1 | <acount-id> | OneCloud/janus-k8s-user | arn:aws:iam::<acount-id>:role/OneCloud/janus-k8s-user |
+---+--------------+-------------------------+--------------------------------------------------------+
? Pick a role from above list: 1
Credentials saved successfully with profile name - saml
To use these credentials, append the --profile saml option at the end of each aws cli command.
```

Check the aws credential created for you by using the following command.
```
cat ~/.aws/credentials 
```

# Set up kubernetes (kubectl) context
Use the following command to set up the aws context

```
aws --profile $PROFILE eks update-kubeconfig --name $CLUSTER --region $AWS_REGION
```
Here are the sample values

    PROFILE=saml
    AWS_REGION=us-east-1
    CLUSTER=eks-janus-test-east1-eksCluster-3127f11

# Validate cluster 
```
kubectl -n entos-ripple-dev   get pods
NAME                              READY   STATUS    RESTARTS   AGE
otel-collector-6cc6bdbd86-bgs6k   2/2     Running   0          6h20m
```

# How to create secret and update secret
## Generic instructions for Creating a TLS Certificate for Istio Ingress Gateway

To create a TLS certificate for an Istio Ingress Gateway, you can follow these general steps:

1. **Create or obtain a TLS certificate and private key:**
    - You can use a certificate authority (CA) to issue a certificate or use a self-signed certificate for testing purposes.
    - Make sure you have both the certificate (.crt) and private key (.key) files available.

2. **Create a Kubernetes Secret to store the TLS certificate and private key:**
    ```bash
    kubectl create secret tls my-tls-secret --cert=path/to/your/certificate.crt --key=path/to/your/privatekey.key
    ```
    Replace `my-tls-secret` with a meaningful name for your secret and provide the paths to your certificate and private key files.

3. **Define a Gateway resource with TLS settings:**
    Create or modify a Kubernetes Gateway resource to specify the TLS configuration. Here's an example YAML configuration:
    ```yaml
    apiVersion: networking.istio.io/v1alpha3
    kind: Gateway
    metadata:
      name: my-gateway
    spec:
      selector:
        istio: ingressgateway
      servers:
        - port:
            number: 443
            name: https
            protocol: HTTPS
          hosts:
            - mydomain.com  # Replace with your domain
          tls:
            mode: SIMPLE
            credentialName: my-tls-secret  # Use the name of the secret created earlier
    ```
    Replace `my-gateway`, `mydomain.com`, and `my-tls-secret` with appropriate values for your use case.

4. **Apply the Gateway configuration to your cluster:**
    ```bash
    kubectl apply -f gateway-config.yaml
    ```
    Replace `gateway-config.yaml` with the actual filename of your Gateway configuration file.

5. **Ensure that your DNS records point to the Istio Ingress Gateway's IP address or hostname.**

Once you've completed these steps, your Istio Ingress Gateway should be configured to terminate TLS traffic using the certificate provided in the secret.

##  Creating a TLS Certificate in Comcast environement
Comcast environment has internal tools to create this secret for the admin and it expects to configure a yaml file as shown below

```yaml
#this is used to create the letsencrypt cert for the prod instance
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: cert-entos-obs.svc-dev.thor.comcast.com
  namespace: istio-system
spec:
  secretName:  cert-entos-obs.svc-dev.thor.comcast.com
  issuerRef:
    name: letsencrypt-dev-delegated
    kind: ClusterIssuer
  commonName: entos-observability.svc-dev.thor.comcast.com
  dnsNames:
    - entos-observability.svc-dev.thor.comcast.com
```
