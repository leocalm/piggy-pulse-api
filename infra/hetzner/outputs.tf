output "server_name" {
  description = "Server name"
  value       = hcloud_server.api.name
}

output "server_ipv4" {
  description = "Public IPv4"
  value       = hcloud_server.api.ipv4_address
}

output "server_ipv6" {
  description = "Public IPv6"
  value       = hcloud_server.api.ipv6_address
}

output "api_domain" {
  description = "Domain expected to point to this server"
  value       = var.api_domain
}

output "cloudflare_dns_managed" {
  description = "Whether API DNS records are managed by this Terraform stack"
  value       = var.manage_cloudflare_dns
}

output "cloudflare_proxied" {
  description = "Whether Cloudflare proxy is enabled for API records"
  value       = var.cloudflare_proxied
}

output "wireguard_server_public_key_command" {
  description = "Run this after Ansible configures WireGuard to fetch the server public key"
  value       = "ssh root@${hcloud_server.api.ipv4_address} 'cat /etc/wireguard/server_public.key'"
}

output "cloudflare_api_a_record_id" {
  description = "Cloudflare A record ID for the API domain (if managed)"
  value       = try(cloudflare_dns_record.api_ipv4[0].id, null)
}

output "cloudflare_api_aaaa_record_id" {
  description = "Cloudflare AAAA record ID for the API domain (if managed)"
  value       = try(cloudflare_dns_record.api_ipv6[0].id, null)
}
