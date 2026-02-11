# Fresh Rebuild Plan (Terraform + Ansible)

## Goal

Build a clean production host with reproducible IaC and no accidental VM recreation for break-glass.

## Phase 0: Backups and Prep

1. Back up database and current `.env` secrets from old VM.
2. Prepare `terraform.tfvars` for a new environment name (for example `prod2`).
3. Prepare Ansible inventory and vault files.

## Phase 1: Provision Infrastructure (Terraform)

Use initial bootstrap mode:
- `ssh_mode = "admin_cidr"`
- `enable_breakglass_ssh = false`
- `admin_ssh_allowed_cidrs = ["<your-static-ip>/32"]`

Run:

```bash
cd infra/hetzner
terraform init
terraform plan
terraform apply
```

Confirm plan did not include unexpected replacement actions.

## Phase 2: Configure Host + Deploy App (Ansible)

Run from `ansible/`:

```bash
ansible-playbook site.yml --ask-vault-pass
```

This configures:
- deploy user + SSH hardening
- nftables + fail2ban
- WireGuard
- Docker
- app deploy + systemd service

Validate:
- `wg show`
- `systemctl status piggypulse-stack.service`
- `ss -lntup`

## Phase 3: Switch SSH to VPN-Only

1. Terraform: set `ssh_mode = "vpn_only"`; apply.
2. Ansible: set `security_ssh_mode: vpn_only`; run playbook again.

## Phase 4: DNS/Proxy Finalization

1. Keep `cloudflare_proxied=false` until origin certificate is healthy.
2. Validate `https://api.piggy-pulse.com/api/v1/health`.
3. Set `cloudflare_proxied=true`; apply Terraform.

## Break-Glass Procedure (No Rebuild)

Emergency direct SSH requires both layers:

1. Terraform firewall: `enable_breakglass_ssh = true` and apply.
2. Host firewall (Ansible): `security_enable_breakglass_ssh: true` and run playbook.
3. Revert both settings after incident.

Changing only `enable_breakglass_ssh` should not recreate the VM.

## Safety Rule Before Every Apply

Always check `terraform plan` and confirm `hcloud_server.api` is not `-/+` (replace) unless replacement is intentional.
