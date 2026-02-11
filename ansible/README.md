# Ansible Operations (PiggyPulse)

This directory manages mutable production state on the Hetzner VM:
- user + SSH hardening
- nftables + fail2ban
- WireGuard configuration
- Docker installation
- app deployment (`deploy/production` stack)

Terraform should provision infrastructure only. Use Ansible for day-2 operations.

## Prerequisites

- Ansible installed locally
- SSH access to the VM (initially from admin CIDR)
- Terraform already applied from `infra/hetzner`

## Layout

- `site.yml`: main playbook
- `inventories/production/hosts.yml`: target hosts
- `inventories/production/group_vars/all/all.yml`: non-secret settings
- `inventories/production/group_vars/all/vault.yml`: encrypted secrets (Ansible Vault)

## Configure Inventory

1. Update host IP and SSH key in `inventories/production/hosts.yml`.
2. Update non-secret settings in `inventories/production/group_vars/all/all.yml`.
3. Set `piggypulse_deploy_mode`:
   - `image_only` (recommended): no git checkout on the server, deploy files are copied from this repo checkout.
   - `repo`: server performs `git clone/pull` before deploy.
4. Set `piggypulse_api_image` and `piggypulse_cron_image` to GHCR digest refs published by CI.
5. Source digests from the `image-digests` artifact in the `Publish Container Images` workflow run.
6. Use `ansible-image-vars.yml` from that artifact to update:
   - `piggypulse_api_image`
   - `piggypulse_cron_image`
7. If `piggypulse_deploy_mode: repo` and the repo is private, set these in Vault:
   - `vault_piggypulse_repo_username` (your GitHub username)
   - `vault_piggypulse_repo_token`
   and keep `piggypulse_repo_url` as HTTPS.
8. Set `piggypulse_repo_requires_auth: true` for private repositories (repo mode only).
9. If GHCR packages are private, set:
   - `piggypulse_ghcr_requires_auth: true`
   - `vault_piggypulse_ghcr_username`
   - `vault_piggypulse_ghcr_token` (`read:packages` scope)

## Vault Setup

Create encrypted secrets file from the example:

```bash
cd ansible
cp inventories/production/group_vars/all/vault.yml.example inventories/production/group_vars/all/vault.yml
ansible-vault encrypt inventories/production/group_vars/all/vault.yml
```

Or with make:

```bash
cd ansible
make vault-init
```

Edit encrypted secrets:

```bash
ansible-vault edit inventories/production/group_vars/all/vault.yml
```

Or:

```bash
make vault-edit
```

If `piggypulse_deploy_mode: repo` and the repository is private, add repo credentials in `vault.yml`:
- `vault_piggypulse_repo_username`
- `vault_piggypulse_repo_token`
  - token should have read access to repository contents (Fine-grained PAT: `Contents: Read`).

If GHCR images are private, also add:
- `vault_piggypulse_ghcr_username`
- `vault_piggypulse_ghcr_token`
  - token should have package pull access (`read:packages`).

Run playbook with vault prompt:

```bash
ansible-playbook site.yml --ask-vault-pass
```

Or:

```bash
make run
```

## First Run Flow (Recommended)

1. In Terraform, provision with `ssh_mode = "admin_cidr"` and your static IP in `admin_ssh_allowed_cidrs`.
2. Run Ansible once as `root` (`ansible_user: root`) to configure deploy user, host firewall, WireGuard, Docker, and app.
3. Verify VPN connectivity.
4. Change `ansible_user` in inventory to `deploy`.
5. Switch to VPN-only mode:
   - Terraform: `ssh_mode = "vpn_only"`
   - Ansible vars: `security_ssh_mode: vpn_only`
   - Apply Terraform then re-run Ansible.

## Break-Glass SSH (No VM Recreation)

Direct SSH in emergency requires both layers open:

1. Terraform layer (Hetzner firewall):
   - `enable_breakglass_ssh = true`
   - `admin_ssh_allowed_cidrs = ["<your-ip>/32"]`
   - `terraform apply`
2. Host firewall layer (Ansible nftables):
   - set `security_enable_breakglass_ssh: true`
   - `ansible-playbook site.yml --ask-vault-pass`

Disable both after incident.

## Useful Commands

```bash
cd ansible
ansible all -m ping --ask-vault-pass
ansible-playbook site.yml --ask-vault-pass
ansible-playbook site.yml --tags security,wireguard --ask-vault-pass
```

## Rollback

1. Set `piggypulse_api_image` and `piggypulse_cron_image` back to a previous digest pair.
2. Run `make run`.

## GitHub Actions Deploy

Automated deploy workflow: `.github/workflows/deploy-production.yml`.

- Auto trigger: after successful `Publish Container Images` run on `main`
- Manual trigger: `workflow_dispatch` with explicit image refs
- Runner requirement: self-hosted Linux runner with network access to your production host (`10.66.0.1` for VPN-only mode)

Required repository/environment secrets:

- `ANSIBLE_VAULT_PASSWORD`
- `PROD_SSH_KNOWN_HOSTS`
  - build it from verified host keys (example: `ssh-keyscan -H <host-or-vpn-ip>`)

Optional secret:

- `PROD_SSH_PRIVATE_KEY`
  - only needed if the self-hosted runner does not already have the SSH key referenced by inventory
