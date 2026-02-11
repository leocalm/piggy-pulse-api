resource "cloudflare_dns_record" "api_ipv4" {
  count = var.manage_cloudflare_dns ? 1 : 0

  zone_id = var.cloudflare_zone_id
  name    = var.api_domain
  type    = "A"
  content = hcloud_server.api.ipv4_address
  proxied = var.cloudflare_proxied
  ttl     = var.cloudflare_proxied ? 1 : var.cloudflare_ttl
  comment = "Managed by Terraform (${var.project_name}-${var.environment})"
}

resource "cloudflare_dns_record" "api_ipv6" {
  count = var.manage_cloudflare_dns && var.create_ipv6_dns_record ? 1 : 0

  zone_id = var.cloudflare_zone_id
  name    = var.api_domain
  type    = "AAAA"
  content = local.server_ipv6_address
  proxied = var.cloudflare_proxied
  ttl     = var.cloudflare_proxied ? 1 : var.cloudflare_ttl
  comment = "Managed by Terraform (${var.project_name}-${var.environment})"
}
