locals {
  stack_name = "${var.project_name}-${var.environment}"

  common_labels = merge(
    {
      project     = var.project_name
      environment = var.environment
      managed_by  = "terraform"
    },
    var.labels,
  )

  admin_ssh_ipv4_cidrs = [for cidr in var.admin_ssh_allowed_cidrs : cidr if !strcontains(cidr, ":")]
  admin_ssh_ipv6_cidrs = [for cidr in var.admin_ssh_allowed_cidrs : cidr if strcontains(cidr, ":")]
  ssh_public_keys_map  = { for key in var.ssh_public_keys : substr(sha1(key), 0, 12) => key }

  enable_direct_ssh_rule = (var.ssh_mode == "admin_cidr" || var.enable_breakglass_ssh) && length(var.admin_ssh_allowed_cidrs) > 0
  server_ipv6_address    = split("/", hcloud_server.api.ipv6_address)[0]
}

resource "terraform_data" "input_validation" {
  input = "validated"

  lifecycle {
    precondition {
      condition     = !(var.ssh_mode == "admin_cidr" && length(var.admin_ssh_allowed_cidrs) == 0)
      error_message = "ssh_mode=admin_cidr requires admin_ssh_allowed_cidrs."
    }

    precondition {
      condition     = !(var.ssh_mode == "vpn_only" && !var.enable_wireguard)
      error_message = "ssh_mode=vpn_only requires enable_wireguard=true."
    }

    precondition {
      condition     = !(var.enable_breakglass_ssh && length(var.admin_ssh_allowed_cidrs) == 0)
      error_message = "enable_breakglass_ssh=true requires admin_ssh_allowed_cidrs."
    }

    precondition {
      condition     = !(var.manage_cloudflare_dns && var.cloudflare_zone_id == "")
      error_message = "manage_cloudflare_dns=true requires cloudflare_zone_id."
    }
  }
}

resource "hcloud_firewall" "edge" {
  name   = "${local.stack_name}-edge"
  labels = local.common_labels

  rule {
    direction  = "in"
    protocol   = "tcp"
    port       = "443"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  rule {
    direction  = "in"
    protocol   = "icmp"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  dynamic "rule" {
    for_each = var.enable_wireguard ? [1] : []

    content {
      direction  = "in"
      protocol   = "udp"
      port       = tostring(var.wireguard_port)
      source_ips = var.wireguard_source_cidrs
    }
  }

  dynamic "rule" {
    for_each = local.enable_direct_ssh_rule ? [1] : []

    content {
      direction  = "in"
      protocol   = "tcp"
      port       = "22"
      source_ips = var.admin_ssh_allowed_cidrs
    }
  }
}

resource "hcloud_ssh_key" "admin" {
  for_each = local.ssh_public_keys_map

  name       = "${local.stack_name}-admin-${each.key}"
  public_key = each.value
  labels     = local.common_labels
}

resource "hcloud_server" "api" {
  name               = "${local.stack_name}-api"
  server_type        = var.server_type
  image              = var.image
  location           = var.location
  backups            = var.enable_backups
  delete_protection  = var.delete_protection
  rebuild_protection = var.rebuild_protection
  labels             = local.common_labels
  firewall_ids       = [hcloud_firewall.edge.id]
  ssh_keys           = [for key_id in sort(keys(hcloud_ssh_key.admin)) : hcloud_ssh_key.admin[key_id].id]

  public_net {
    ipv4_enabled = true
    ipv6_enabled = true
  }

  user_data = templatefile("${path.module}/cloud-init.yaml.tftpl", {
    server_hostname = "${local.stack_name}-api"
    timezone        = var.timezone
  })

  depends_on = [terraform_data.input_validation]
}
