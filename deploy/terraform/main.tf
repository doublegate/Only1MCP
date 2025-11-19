# Only1MCP Terraform Configuration
# Deploys Only1MCP to AWS with ECS Fargate

terraform {
  required_version = ">= 1.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Backend configuration for state storage
  backend "s3" {
    bucket         = "only1mcp-terraform-state"
    key            = "production/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "only1mcp-terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "Only1MCP"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

# VPC Module
module "vpc" {
  source = "./modules/vpc"

  environment         = var.environment
  vpc_cidr            = var.vpc_cidr
  availability_zones  = var.availability_zones
  private_subnet_cidrs = var.private_subnet_cidrs
  public_subnet_cidrs  = var.public_subnet_cidrs
}

# Security Groups
module "security" {
  source = "./modules/security"

  environment = var.environment
  vpc_id      = module.vpc.vpc_id
}

# Application Load Balancer
module "alb" {
  source = "./modules/alb"

  environment        = var.environment
  vpc_id             = module.vpc.vpc_id
  public_subnet_ids  = module.vpc.public_subnet_ids
  security_group_ids = [module.security.alb_security_group_id]
  certificate_arn    = var.ssl_certificate_arn
}

# ECS Cluster
module "ecs" {
  source = "./modules/ecs"

  environment          = var.environment
  vpc_id               = module.vpc.vpc_id
  private_subnet_ids   = module.vpc.private_subnet_ids
  security_group_ids   = [module.security.ecs_security_group_id]
  target_group_arn     = module.alb.target_group_arn

  # Container configuration
  container_image      = var.container_image
  container_port       = 8080
  desired_count        = var.desired_count
  cpu                  = var.task_cpu
  memory               = var.task_memory

  # Auto-scaling
  min_capacity         = var.min_capacity
  max_capacity         = var.max_capacity
  target_cpu_utilization = 70
  target_memory_utilization = 80
}

# ElastiCache Redis for caching
module "redis" {
  source = "./modules/redis"

  environment        = var.environment
  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnet_ids
  security_group_ids = [module.security.redis_security_group_id]
  node_type          = var.redis_node_type
  num_cache_nodes    = var.redis_num_nodes
}

# RDS for persistent storage (optional)
module "rds" {
  source = "./modules/rds"

  environment        = var.environment
  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnet_ids
  security_group_ids = [module.security.rds_security_group_id]
  instance_class     = var.rds_instance_class
  allocated_storage  = var.rds_allocated_storage
  database_name      = "only1mcp"
  master_username    = var.db_username
  master_password    = var.db_password

  count = var.enable_rds ? 1 : 0
}

# CloudWatch for monitoring
module "cloudwatch" {
  source = "./modules/cloudwatch"

  environment       = var.environment
  ecs_cluster_name  = module.ecs.cluster_name
  ecs_service_name  = module.ecs.service_name
  alb_arn_suffix    = module.alb.alb_arn_suffix
  target_group_arn_suffix = module.alb.target_group_arn_suffix

  # Alarm SNS topic
  alarm_email       = var.alarm_email
}

# S3 bucket for configuration and logs
resource "aws_s3_bucket" "config" {
  bucket = "${var.environment}-only1mcp-config"
}

resource "aws_s3_bucket_versioning" "config" {
  bucket = aws_s3_bucket.config.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "config" {
  bucket = aws_s3_bucket.config.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# Secrets Manager for sensitive configuration
resource "aws_secretsmanager_secret" "jwt_secret" {
  name        = "${var.environment}/only1mcp/jwt-secret"
  description = "JWT secret for Only1MCP authentication"
}

resource "aws_secretsmanager_secret_version" "jwt_secret" {
  secret_id     = aws_secretsmanager_secret.jwt_secret.id
  secret_string = var.jwt_secret
}

# IAM role for ECS tasks
resource "aws_iam_role" "ecs_task_execution" {
  name = "${var.environment}-only1mcp-ecs-task-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution" {
  role       = aws_iam_role.ecs_task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Output values
output "alb_dns_name" {
  description = "DNS name of the Application Load Balancer"
  value       = module.alb.alb_dns_name
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster"
  value       = module.ecs.cluster_name
}

output "redis_endpoint" {
  description = "Redis cluster endpoint"
  value       = module.redis.endpoint
}

output "rds_endpoint" {
  description = "RDS instance endpoint"
  value       = var.enable_rds ? module.rds[0].endpoint : null
}
