variable "hcloud_token" {
  type        = string
  description = "Hetzner Cloud API token"
  sensitive   = true
}

variable "project_name" {
  type        = string
  description = "Project slug used for naming"
  default     = "piggy-pulse"
}

variable "environment" {
  type        = string
  description = "Environment slug"
  default     = "prod"
}

variable "server_type" {
  type        = string
  description = "Hetzner server type"
  default     = "cpx21"
}

variable "image" {
  type        = string
  description = "Hetzner image"
  default     = "ubuntu-24.04"
}

variable "location" {
  type        = string
  description = "Hetzner location"
  default     = "nbg1"
}

variable "enable_backups" {
  type        = bool
  description = "Enable Hetzner snapshot backups"
  default     = true
}

variable "delete_protection" {
  type        = bool
  description = "Protect server from accidental deletion"
  default     = true
}

variable "rebuild_protection" {
  type        = bool
  description = "Protect server from accidental rebuild"
  default     = true
}

variable "api_domain" {
  type        = string
  description = "API domain that will point to this VM"
  default     = "api.piggy-pulse.com"
}

variable "timezone" {
  type        = string
  description = "Server timezone"
  default     = "UTC"
}

variable "ssh_public_keys" {
  type        = list(string)
  description = "Public SSH keys registered in Hetzner and injected for initial root SSH access"

  validation {
    condition     = length(var.ssh_public_keys) > 0
    error_message = "Set at least one SSH public key."
  }
}

variable "ssh_mode" {
  type        = string
  description = "SSH exposure mode: vpn_only or admin_cidr"
  default     = "vpn_only"

  validation {
    condition     = contains(["vpn_only", "admin_cidr"], var.ssh_mode)
    error_message = "ssh_mode must be vpn_only or admin_cidr."
  }
}

variable "admin_ssh_allowed_cidrs" {
  type        = list(string)
  description = "Allowed source CIDRs for direct SSH when ssh_mode=admin_cidr or breakglass is enabled"
  default     = []
}

variable "enable_breakglass_ssh" {
  type        = bool
  description = "Allow direct SSH from admin_ssh_allowed_cidrs even when ssh_mode=vpn_only"
  default     = false
}

variable "enable_wireguard" {
  type        = bool
  description = "Enable WireGuard VPN endpoint"
  default     = true
}

variable "wireguard_port" {
  type        = number
  description = "WireGuard UDP listen port"
  default     = 51820
}

variable "wireguard_source_cidrs" {
  type        = list(string)
  description = "Source CIDRs allowed to hit the WireGuard UDP port"
  default     = ["0.0.0.0/0", "::/0"]
}

variable "labels" {
  type        = map(string)
  description = "Extra labels for Hetzner resources"
  default     = {}
}

variable "manage_cloudflare_dns" {
  type        = bool
  description = "Whether Terraform should manage Cloudflare DNS records for the API domain"
  default     = true
}

variable "cloudflare_api_token" {
  type        = string
  description = "Cloudflare API token with DNS edit permissions for the target zone (optional if CLOUDFLARE_API_TOKEN env var is set)"
  sensitive   = true
  default     = ""
}

variable "cloudflare_zone_id" {
  type        = string
  description = "Cloudflare zone ID for piggy-pulse.com"
  default     = ""
}

variable "cloudflare_proxied" {
  type        = bool
  description = "Whether the API DNS records are proxied through Cloudflare"
  default     = false
}

variable "cloudflare_ttl" {
  type        = number
  description = "TTL for DNS records when not proxied"
  default     = 300

  validation {
    condition     = var.cloudflare_ttl == 1 || (var.cloudflare_ttl >= 60 && var.cloudflare_ttl <= 86400)
    error_message = "cloudflare_ttl must be 1 (auto) or between 60 and 86400 seconds."
  }
}

variable "create_ipv6_dns_record" {
  type        = bool
  description = "Create an AAAA record for the API domain"
  default     = true
}
