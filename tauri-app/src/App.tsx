import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/dialog';
import { Play, Square, Settings, Server, Activity, AlertCircle } from 'lucide-react';
import { Line Line Chart, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

// Types
interface ProxyStatus {
  running: boolean;
  uptime_seconds?: number;
  version: string;
  config_path?: string;
}

interface MetricsData {
  requests_total: number;
  requests_per_second: number;
  avg_latency_ms: number;
  active_connections: number;
  cache_hit_rate: number;
  error_rate: number;
}

interface ServerInfo {
  id: string;
  name: string;
  status: string;
  health: string;
  requests: number;
}

export default function App() {
  const [status, setStatus] = useState<ProxyStatus | null>(null);
  const [metrics, setMetrics] = useState<MetricsData | null>(null);
  const [servers, setServers] = useState<ServerInfo[]>([]);
  const [metricsHistory, setMetricsHistory] = useState<any[]>([]);
  const [configPath, setConfigPath] = useState('');
  const [loading, setLoading] = useState(false);

  // Load initial status
  useEffect(() => {
    loadStatus();

    // Listen for metrics updates
    const unlisten = listen<MetricsData>('metrics-update', (event) => {
      setMetrics(event.payload);

      // Add to history (keep last 60 data points)
      setMetricsHistory((prev) => {
        const newHistory = [
          ...prev,
          {
            time: new Date().toLocaleTimeString(),
            rps: event.payload.requests_per_second,
            latency: event.payload.avg_latency_ms,
            connections: event.payload.active_connections,
          },
        ].slice(-60);
        return newHistory;
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadStatus = async () => {
    try {
      const statusData = await invoke<ProxyStatus>('get_status');
      setStatus(statusData);

      if (statusData.running) {
        const serversData = await invoke<ServerInfo[]>('get_servers');
        setServers(serversData);

        const metricsData = await invoke<MetricsData>('get_metrics');
        setMetrics(metricsData);
      }
    } catch (error) {
      console.error('Failed to load status:', error);
    }
  };

  const handleStart = async () => {
    if (!configPath) {
      const selected = await open({
        filters: [
          {
            name: 'Config',
            extensions: ['yaml', 'yml', 'toml'],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      setConfigPath(selected);
    }

    try {
      setLoading(true);
      await invoke('start_proxy', { configPath });
      await loadStatus();
    } catch (error) {
      console.error('Failed to start proxy:', error);
      alert(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleStop = async () => {
    try {
      setLoading(true);
      await invoke('stop_proxy');
      await loadStatus();
      setMetrics(null);
      setServers([]);
      setMetricsHistory([]);
    } catch (error) {
      console.error('Failed to stop proxy:', error);
    } finally {
      setLoading(false);
    }
  };

  const isRunning = status?.running || false;

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">Only1MCP Desktop</h1>
              <p className="text-sm text-gray-500">v{status?.version}</p>
            </div>

            <div className="flex items-center space-x-4">
              <div className="flex items-center">
                <div className={`h-3 w-3 rounded-full mr-2 ${isRunning ? 'bg-green-500' : 'bg-gray-300'}`} />
                <span className="text-sm font-medium text-gray-700">
                  {isRunning ? 'Running' : 'Stopped'}
                </span>
              </div>

              {isRunning ? (
                <button
                  onClick={handleStop}
                  disabled={loading}
                  className="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-50"
                >
                  <Square className="h-4 w-4 mr-2" />
                  Stop
                </button>
              ) : (
                <button
                  onClick={handleStart}
                  disabled={loading}
                  className="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50"
                >
                  <Play className="h-4 w-4 mr-2" />
                  Start
                </button>
              )}

              <button className="inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50">
                <Settings className="h-4 w-4 mr-2" />
                Settings
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {!isRunning ? (
          <div className="text-center py-12">
            <Server className="mx-auto h-12 w-12 text-gray-400" />
            <h3 className="mt-2 text-sm font-medium text-gray-900">Proxy Not Running</h3>
            <p className="mt-1 text-sm text-gray-500">
              Click Start to begin proxying MCP servers
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {/* Metrics Cards */}
            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
              <MetricCard
                title="Requests/sec"
                value={metrics?.requests_per_second.toFixed(2) || '0'}
                icon={<Activity className="h-6 w-6 text-blue-600" />}
              />
              <MetricCard
                title="Avg Latency"
                value={`${metrics?.avg_latency_ms.toFixed(2) || '0'}ms`}
                icon={<Activity className="h-6 w-6 text-green-600" />}
              />
              <MetricCard
                title="Active Connections"
                value={metrics?.active_connections.toString() || '0'}
                icon={<Server className="h-6 w-6 text-purple-600" />}
              />
              <MetricCard
                title="Error Rate"
                value={`${((metrics?.error_rate || 0) * 100).toFixed(2)}%`}
                icon={<AlertCircle className="h-6 w-6 text-red-600" />}
              />
            </div>

            {/* Charts */}
            {metricsHistory.length > 0 && (
              <div className="bg-white p-6 rounded-lg shadow">
                <h2 className="text-lg font-medium text-gray-900 mb-4">Performance Metrics</h2>
                <ResponsiveContainer width="100%" height={300}>
                  <LineChart data={metricsHistory}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="time" />
                    <YAxis yAxisId="left" />
                    <YAxis yAxisId="right" orientation="right" />
                    <Tooltip />
                    <Legend />
                    <Line
                      yAxisId="left"
                      type="monotone"
                      dataKey="rps"
                      stroke="#3b82f6"
                      name="Requests/sec"
                      strokeWidth={2}
                    />
                    <Line
                      yAxisId="right"
                      type="monotone"
                      dataKey="latency"
                      stroke="#10b981"
                      name="Latency (ms)"
                      strokeWidth={2}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            )}

            {/* Servers Table */}
            <div className="bg-white shadow overflow-hidden sm:rounded-lg">
              <div className="px-4 py-5 sm:px-6">
                <h2 className="text-lg font-medium text-gray-900">MCP Servers</h2>
              </div>
              <div className="border-t border-gray-200">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                        Name
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                        Status
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                        Health
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                        Requests
                      </th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {servers.map((server) => (
                      <tr key={server.id}>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                          {server.name}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                            {server.status}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                          {server.health}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                          {server.requests}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Metric Card Component
function MetricCard({ title, value, icon }: { title: string; value: string; icon: React.ReactNode }) {
  return (
    <div className="bg-white overflow-hidden shadow rounded-lg">
      <div className="p-5">
        <div className="flex items-center">
          <div className="flex-shrink-0">{icon}</div>
          <div className="ml-5 w-0 flex-1">
            <dl>
              <dt className="text-sm font-medium text-gray-500 truncate">{title}</dt>
              <dd className="text-2xl font-semibold text-gray-900">{value}</dd>
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}
