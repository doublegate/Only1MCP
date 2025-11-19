# Only1MCP GCP Terraform Configuration

terraform {
  required_version = ">= 1.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# VPC Network
resource "google_compute_network" "only1mcp_network" {
  name                    = "${var.environment}-only1mcp-network"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "only1mcp_subnet" {
  name          = "${var.environment}-only1mcp-subnet"
  ip_cidr_range = var.subnet_cidr
  region        = var.region
  network       = google_compute_network.only1mcp_network.id

  secondary_ip_range {
    range_name    = "pods"
    ip_cidr_range = var.pods_cidr
  }

  secondary_ip_range {
    range_name    = "services"
    ip_cidr_range = var.services_cidr
  }
}

# GKE Cluster
resource "google_container_cluster" "only1mcp_cluster" {
  name     = "${var.environment}-only1mcp-cluster"
  location = var.region

  # Remove default node pool
  remove_default_node_pool = true
  initial_node_count       = 1

  network    = google_compute_network.only1mcp_network.name
  subnetwork = google_compute_subnetwork.only1mcp_subnet.name

  ip_allocation_policy {
    cluster_secondary_range_name  = "pods"
    services_secondary_range_name = "services"
  }

  addons_config {
    http_load_balancing {
      disabled = false
    }
    horizontal_pod_autoscaling {
      disabled = false
    }
  }

  workload_identity_config {
    workload_pool = "${var.project_id}.svc.id.goog"
  }

  monitoring_config {
    enable_components = ["SYSTEM_COMPONENTS", "WORKLOADS"]
  }

  logging_config {
    enable_components = ["SYSTEM_COMPONENTS", "WORKLOADS"]
  }
}

# Node Pool
resource "google_container_node_pool" "primary_nodes" {
  name       = "${var.environment}-primary-pool"
  location   = var.region
  cluster    = google_container_cluster.only1mcp_cluster.name
  node_count = var.node_count

  autoscaling {
    min_node_count = var.min_node_count
    max_node_count = var.max_node_count
  }

  node_config {
    machine_type = var.machine_type
    disk_size_gb = 50
    disk_type    = "pd-standard"

    oauth_scopes = [
      "https://www.googleapis.com/auth/cloud-platform",
    ]

    labels = {
      environment = var.environment
      app         = "only1mcp"
    }

    workload_metadata_config {
      mode = "GKE_METADATA"
    }
  }

  management {
    auto_repair  = true
    auto_upgrade = true
  }
}

# Cloud SQL (PostgreSQL) for plugin marketplace
resource "google_sql_database_instance" "postgres" {
  count            = var.enable_marketplace ? 1 : 0
  name             = "${var.environment}-only1mcp-db"
  database_version = "POSTGRES_15"
  region           = var.region

  settings {
    tier              = var.db_tier
    availability_type = var.db_availability_type
    disk_size         = 20
    disk_autoresize   = true

    backup_configuration {
      enabled                        = true
      point_in_time_recovery_enabled = true
    }

    ip_configuration {
      ipv4_enabled    = true
      private_network = google_compute_network.only1mcp_network.id
    }

    database_flags {
      name  = "max_connections"
      value = "100"
    }
  }

  deletion_protection = var.environment == "production"
}

resource "google_sql_database" "marketplace_db" {
  count    = var.enable_marketplace ? 1 : 0
  name     = "marketplace"
  instance = google_sql_database_instance.postgres[0].name
}

resource "google_sql_user" "marketplace_user" {
  count    = var.enable_marketplace ? 1 : 0
  name     = "marketplace"
  instance = google_sql_database_instance.postgres[0].name
  password = var.db_password
}

# Redis (Memorystore) for caching
resource "google_redis_instance" "cache" {
  name           = "${var.environment}-only1mcp-cache"
  tier           = var.redis_tier
  memory_size_gb = var.redis_memory_gb
  region         = var.region

  authorized_network = google_compute_network.only1mcp_network.id

  redis_version = "REDIS_6_X"

  display_name = "Only1MCP Cache"
}

# Cloud Storage bucket for plugins
resource "google_storage_bucket" "plugins" {
  count    = var.enable_marketplace ? 1 : 0
  name     = "${var.project_id}-only1mcp-plugins"
  location = var.region

  uniform_bucket_level_access = true

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      num_newer_versions = 3
    }
    action {
      type = "Delete"
    }
  }
}

# Service Account for workloads
resource "google_service_account" "only1mcp_sa" {
  account_id   = "${var.environment}-only1mcp"
  display_name = "Only1MCP Service Account"
}

resource "google_project_iam_member" "only1mcp_sa_roles" {
  for_each = toset([
    "roles/cloudtrace.agent",
    "roles/monitoring.metricWriter",
    "roles/logging.logWriter",
  ])

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.only1mcp_sa.email}"
}

# Allow service account to access Cloud SQL
resource "google_project_iam_member" "sql_client" {
  count   = var.enable_marketplace ? 1 : 0
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.only1mcp_sa.email}"
}

# Allow service account to access Storage
resource "google_storage_bucket_iam_member" "plugins_admin" {
  count  = var.enable_marketplace ? 1 : 0
  bucket = google_storage_bucket.plugins[0].name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.only1mcp_sa.email}"
}

# Static IP for Load Balancer
resource "google_compute_global_address" "only1mcp_ip" {
  name = "${var.environment}-only1mcp-ip"
}

# Outputs
output "cluster_name" {
  value = google_container_cluster.only1mcp_cluster.name
}

output "cluster_endpoint" {
  value     = google_container_cluster.only1mcp_cluster.endpoint
  sensitive = true
}

output "redis_host" {
  value = google_redis_instance.cache.host
}

output "redis_port" {
  value = google_redis_instance.cache.port
}

output "db_connection_name" {
  value = var.enable_marketplace ? google_sql_database_instance.postgres[0].connection_name : null
}

output "static_ip" {
  value = google_compute_global_address.only1mcp_ip.address
}

output "service_account_email" {
  value = google_service_account.only1mcp_sa.email
}

output "plugins_bucket" {
  value = var.enable_marketplace ? google_storage_bucket.plugins[0].name : null
}
