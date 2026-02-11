# Hetzner Provisioning (Terraform)

This stack provisions infrastructure only:
- Hetzner VM
- Hetzner Cloud firewall
- Cloudflare DNS (`api.piggy-pulse.com`)
- Minimal cloud-init bootstrap

Host hardening and application deployment are managed by Ansible in `ansible/`.

## What Terraform Manages

- Public inbound firewall rules:
  - `443/tcp`
  - `51820/udp` (when `enable_wireguard = true`)
  - `22/tcp` only when `ssh_mode = "admin_cidr"` or `enable_breakglass_ssh = true`
- Delete/rebuild protection
- Backups
- DNS A/AAAA records in Cloudflare

Break-glass toggle is firewall-only and does not modify `user_data`.

## Prerequisites

- Terraform >= 1.6
- Hetzner Cloud API token
- Cloudflare zone + API token (or `CLOUDFLARE_API_TOKEN` env)
- At least one SSH public key

## Configure

```bash
cd infra/hetzner
cp terraform.tfvars.example terraform.tfvars
```

Set at minimum:
- `hcloud_token`
- `cloudflare_zone_id`
- `ssh_public_keys`
- `admin_ssh_allowed_cidrs`

## Initial Provisioning Mode

For first bootstrap, use:
- `ssh_mode = "admin_cidr"`
- `enable_breakglass_ssh = false`

This allows Ansible to connect and configure host-level controls.

## Apply

```bash
terraform init
terraform plan
terraform apply
```

## Then Run Ansible

Immediately run Ansible hardening/deploy from `ansible/README.md`.

## Switch to VPN-Only

After Ansible configures WireGuard and host firewall:
1. Set Terraform `ssh_mode = "vpn_only"`
2. Apply Terraform
3. Re-run Ansible with `security_ssh_mode: vpn_only`

## Emergency Break-Glass (No Rebuild)

1. Set `enable_breakglass_ssh = true`
2. `terraform apply`
3. Open host-level break-glass in Ansible (`security_enable_breakglass_ssh: true`)
4. Revert both after incident
