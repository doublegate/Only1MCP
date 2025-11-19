# Google Cloud Platform (GCP) Deployment

Deploy Only1MCP to GCP using Cloud Run, GKE, or Compute Engine.

## Deployment Options

### 1. Cloud Run (Serverless)

**Best for**: Low-moderate traffic, automatic scaling, pay-per-use

```bash
# Build and deploy
gcloud run deploy only1mcp \
  --source . \
  --region us-central1 \
  --platform managed \
  --allow-unauthenticated \
  --memory 2Gi \
  --cpu 2 \
  --min-instances 1 \
  --max-instances 100

# Update environment
gcloud run services update only1mcp \
  --update-env-vars "RUST_LOG=info,WORKERS=4"
```

### 2. GKE (Kubernetes)

**Best for**: Production workloads, advanced features, multi-region

```bash
# Create GKE cluster
gcloud container clusters create only1mcp-cluster \
  --region us-central1 \
  --num-nodes 3 \
  --machine-type n2-standard-4 \
  --enable-autoscaling \
  --min-nodes 2 \
  --max-nodes 10

# Deploy
kubectl apply -f kubernetes/deployment.yaml
```

### 3. Compute Engine (VMs)

**Best for**: Full control, custom configurations

See `terraform/gcp/` for Infrastructure as Code.
