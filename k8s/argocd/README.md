# ArgoCD Setup for PayX

## Prerequisites

1. A Kubernetes cluster (Civo, Oracle Cloud, DigitalOcean, etc.)
2. ArgoCD installed on the cluster

## Quick Start: Install ArgoCD

```bash
# Create argocd namespace
kubectl create namespace argocd

# Install ArgoCD
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# Wait for ArgoCD to be ready
kubectl wait --for=condition=available --timeout=300s deployment/argocd-server -n argocd

# Get the initial admin password
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d

# Port forward to access ArgoCD UI
kubectl port-forward svc/argocd-server -n argocd 8080:443
```

Access ArgoCD at https://localhost:8080 (username: `admin`)

## Deploy PayX Application

1. **Update the repository URL** in `application.yaml`:
   ```yaml
   repoURL: https://github.com/benodiwal/payx.git
   ```

2. **Update the image name** in `k8s/base/kustomization.yaml` and `k8s/overlays/production/kustomization.yaml`:
   ```yaml
   images:
     - name: ghcr.io/benodiwal/payx
   ```

3. **Apply the ArgoCD Application**:
   ```bash
   kubectl apply -f k8s/argocd/application.yaml
   ```

4. ArgoCD will automatically sync and deploy PayX!

## GitOps Workflow

1. Push code changes to `main` branch
2. GitHub Actions builds and pushes new Docker image
3. GitHub Actions updates the image tag in `k8s/overlays/production/kustomization.yaml`
4. ArgoCD detects the change and automatically deploys

## Secrets Management

For production, use one of these approaches instead of plain secrets:

- **Sealed Secrets**: https://sealed-secrets.netlify.app/
- **External Secrets Operator**: https://external-secrets.io/
- **SOPS with age/GPG**: https://github.com/mozilla/sops

## Useful Commands

```bash
# Check ArgoCD app status
argocd app get payx

# Manually sync (if auto-sync is disabled)
argocd app sync payx

# View app logs
kubectl logs -n payx -l app.kubernetes.io/name=payx -f

# Check deployment status
kubectl get pods -n payx
```
