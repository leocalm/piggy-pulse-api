# Manual Production Deployment (PiggyPulse)

This guide walks through a full manual deployment, from infrastructure provisioning to a running production app.

Scope:

- One Hetzner VM
- Backend API + cron + Postgres + Redis on the same VM
- Cloudflare DNS for `api.piggy-pulse.com`
- Host hardening and app deploy with Ansible

## 1. Prerequisites

Install locally:

```bash
terraform --version
ansible --version
ansible-playbook --version
wg --version
```

You also need:

- Hetzner Cloud API token
- Cloudflare zone ID for `piggy-pulse.com`
- Cloudflare API token with DNS edit permissions
- One SSH public key for initial server access
- One WireGuard client public key

## 2. Prepare Terraform Variables

```bash
cd infra/hetzner
cp terraform.tfvars.example terraform.tfvars
```

Edit `infra/hetzner/terraform.tfvars`:

- `hcloud_token`
- `cloudflare_zone_id`
- `cloudflare_api_token` (or export `CLOUDFLARE_API_TOKEN`)
- `ssh_public_keys`
- `admin_ssh_allowed_cidrs` (your static IP `/32`)

For first bootstrap, set:

```hcl
ssh_mode              = "admin_cidr"
enable_breakglass_ssh = false
enable_wireguard      = true
```

## 3. Provision Infrastructure

```bash
cd infra/hetzner
terraform init
terraform plan
terraform apply
```

Save output values:

- `server_ipv4`
- `api_domain`

## 4. Prepare Ansible Inventory and Vars

```bash
cd ansible
```

Edit `inventories/production/hosts.yml`:

- `ansible_host` to the server IPv4
- keep first run as `ansible_user: root`

Edit `inventories/production/group_vars/all/all.yml`:

- `security_admin_ssh_allowed_cidrs`
- `security_deploy_ssh_public_keys`
- `wireguard_peers`
- `piggypulse_repo_version`

## 5. Configure Ansible Vault Secrets

```bash
cd ansible
cp inventories/production/group_vars/all/vault.yml.example inventories/production/group_vars/all/vault.yml
ansible-vault encrypt inventories/production/group_vars/all/vault.yml
ansible-vault edit inventories/production/group_vars/all/vault.yml
```

Fill at least:

- `vault_piggypulse_postgres_password`
- `vault_piggypulse_redis_password`
- `vault_piggypulse_rocket_secret_key`
- `vault_piggypulse_email_smtp_host`
- `vault_piggypulse_email_smtp_username`
- `vault_piggypulse_email_smtp_password`

## 6. Build and Publish Images to GHCR

Trigger workflow:

- GitHub Actions -> `Publish Container Images` -> Run workflow

After it finishes, download artifact `image-digests` and open `ansible-image-vars.yml`.

Update `inventories/production/group_vars/all/all.yml`:

- `piggypulse_api_image`
- `piggypulse_cron_image`

Use immutable digest refs (`ghcr.io/...@sha256:...`).

## 7. Run First Ansible Deploy

```bash
cd ansible
ansible-playbook site.yml --ask-vault-pass
```

This will:

- create/harden deploy user and SSH config
- apply nftables + fail2ban
- configure WireGuard
- install Docker
- render production `.env`
- install/start `piggypulse-stack.service`

## 8. Verify App and Services

From your machine:

```bash
curl -I https://api.piggy-pulse.com/api/v1/health
```

On server:

```bash
sudo systemctl status piggypulse-stack.service --no-pager
sudo docker compose -f /opt/piggypulse/budget/deploy/production/docker-compose.yml --env-file /opt/piggypulse/budget/deploy/production/.env ps
ss -lntup
```

Expected public listeners:

- `443/tcp`
- `51820/udp`

## 9. Switch to VPN-Only SSH

After confirming WireGuard works:

1) Terraform (`infra/hetzner/terraform.tfvars`):

```hcl
ssh_mode = "vpn_only"
```

2) Apply:

```bash
cd infra/hetzner
terraform plan
terraform apply
```

3) Ansible vars (`ansible/inventories/production/group_vars/all/all.yml`):

```yaml
security_ssh_mode: vpn_only
```

4) Re-run Ansible:

```bash
cd ansible
ansible-playbook site.yml --ask-vault-pass
```

5) Move inventory host to VPN endpoint and deploy user:

- `ansible_host: 10.66.0.1`
- `ansible_user: deploy`

## 10. Post-Deployment Checks

- Cloudflare SSL/TLS mode: `Full (strict)`
- `BUDGET_API_EXPOSE_DOCS=false`
- CORS only `https://piggy-pulse.com`
- Scheduled DB backups tested with restore
- Monitoring/alerts active for 5xx/auth failures/disk pressure

## 11. Manual Rollback

1. Pick previous digests for API and cron.
2. Update:

- `piggypulse_api_image`
- `piggypulse_cron_image`

3. Run:

```bash
cd ansible
ansible-playbook site.yml --ask-vault-pass
```

## 12. Emergency Break-Glass SSH

Enable both layers:

1) Terraform firewall:

- `enable_breakglass_ssh = true`
- `terraform apply`

2) Host firewall (Ansible):

- `security_enable_breakglass_ssh: true`
- `ansible-playbook site.yml --ask-vault-pass`

Disable both after incident.
