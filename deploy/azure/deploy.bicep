// Azure Bicep template for Only1MCP deployment
param location string = resourceGroup().location
param appName string = 'only1mcp'
param environment string = 'production'

// Container Registry
resource acr 'Microsoft.ContainerRegistry/registries@2023-01-01-preview' = {
  name: '${appName}registry'
  location: location
  sku: {
    name: 'Standard'
  }
  properties: {
    adminUserEnabled: true
  }
}

// Container Instances
resource containerGroup 'Microsoft.ContainerInstance/containerGroups@2023-05-01' = {
  name: '${appName}-containers'
  location: location
  properties: {
    containers: [
      {
        name: appName
        properties: {
          image: '${acr.properties.loginServer}/only1mcp:latest'
          resources: {
            requests: {
              cpu: 2
              memoryInGB: 4
            }
          }
          ports: [
            { port: 8080 }
            { port: 9090 }
          ]
          environmentVariables: [
            { name: 'RUST_LOG', value: 'info' }
            { name: 'ENVIRONMENT', value: environment }
          ]
        }
      }
    ]
    osType: 'Linux'
    restartPolicy: 'Always'
    ipAddress: {
      type: 'Public'
      ports: [
        { protocol: 'TCP', port: 8080 }
        { protocol: 'TCP', port: 9090 }
      ]
      dnsNameLabel: appName
    }
  }
}

output containerGroupId string = containerGroup.id
output fqdn string = containerGroup.properties.ipAddress.fqdn
