/**
 * Unified Views - All pages using ONLY core design system
 * This is how every page should be built
 */

import React, { useEffect, useState } from 'react';
import {
  Page,
  PageHeader,
  Card,
  Metric,
  Table,
  ListItem,
  Button,
  Input,
  StatusDot,
  Badge,
  EmptyState,
  LoadingState,
  Grid,
  COLORS
} from '../design-system/core';
import { invoke } from '@tauri-apps/api/tauri';

// ============================================================================
// SERVICE HEALTH VIEW - Using only core components
// ============================================================================

export const UnifiedHealthView = ({ services, metrics }: any) => {
  const servicesList = services || [];

  const healthColumns = [
    {
      key: 'name',
      label: 'Service',
      render: (item: any) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <StatusDot status={item.error_rate >= 0.05 ? 'error' : item.error_rate >= 0.01 ? 'warning' : 'success'} />
          <span>{item.name}</span>
        </div>
      )
    },
    {
      key: 'trace_count',
      label: 'Traces',
      align: 'right' as const,
      render: (item: any) => <Badge variant="primary">{item.trace_count || 0}</Badge>
    },
    {
      key: 'error_rate',
      label: 'Error Rate',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: (item.error_rate || 0) >= 0.05 ? COLORS.accent.error : COLORS.text.tertiary }}>
          {((item.error_rate || 0) * 100).toFixed(2)}%
        </span>
      )
    },
    {
      key: 'avg_duration',
      label: 'Avg Latency',
      align: 'right' as const,
      render: (item: any) => `${(item.avg_duration || 0).toFixed(0)}ms`
    },
    {
      key: 'p95_duration',
      label: 'P95 Latency',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: COLORS.text.secondary }}>{(item.p95_duration || 0).toFixed(0)}ms</span>
      )
    }
  ];

  // Calculate health statistics
  const healthyServices = servicesList.filter((s: any) => !s.error_rate || s.error_rate < 0.01).length;
  const warningServices = servicesList.filter((s: any) => s.error_rate >= 0.01 && s.error_rate < 0.05).length;
  const criticalServices = servicesList.filter((s: any) => s.error_rate >= 0.05).length;
  const avgResponse = servicesList.length > 0
    ? Math.round(servicesList.reduce((acc: number, s: any) => acc + (s.avg_duration || 0), 0) / servicesList.length)
    : 0;
  const totalTraces = servicesList.reduce((acc: number, s: any) => acc + (s.trace_count || 0), 0);

  return (
    <Page>
      <PageHeader
        title="Service Health"
        subtitle="Real-time service metrics and health status"
        actions={<></>}
        metrics={
          <>
            <Metric label="Total Services" value={servicesList.length} />
            <Metric label="Healthy" value={healthyServices} color="success" />
            <Metric label="Warning" value={warningServices} color="warning" />
            <Metric label="Critical" value={criticalServices} color="error" />
            <Metric label="Avg Response" value={avgResponse > 0 ? `${avgResponse}ms` : 'N/A'} />
          </>
        }
      />

      <div className="urpo-content">
        {servicesList.length === 0 ? (
          <EmptyState
            message="No service health data"
            description="Start sending OTLP data to monitor service health"
          />
        ) : (
          <>
            <Grid cols={4} gap="md">
              <Card>
                <Metric label="Total Traces" value={totalTraces} trend={totalTraces > 100 ? "up" : undefined} />
              </Card>
              <Card>
                <Metric
                  label="Error Rate"
                  value={servicesList.length > 0 ? `${((servicesList.reduce((acc: number, s: any) => acc + (s.error_rate || 0), 0) / servicesList.length) * 100).toFixed(2)}%` : '0.00%'}
                  trend={(servicesList.reduce((acc: number, s: any) => acc + (s.error_rate || 0), 0) / servicesList.length) < 0.01 ? "down" : undefined}
                  color={(servicesList.reduce((acc: number, s: any) => acc + (s.error_rate || 0), 0) / servicesList.length) < 0.01 ? "success" : "error"}
                />
              </Card>
              <Card>
                <Metric label="Avg Latency" value={avgResponse > 0 ? `${avgResponse}ms` : 'N/A'} trend="neutral" />
              </Card>
              <Card>
                <Metric label="Services Monitored" value={servicesList.length} color="primary" />
              </Card>
            </Grid>

            <div className="urpo-section">
              <Table
                data={servicesList}
                columns={healthColumns}
                onRowClick={(item) => console.log('Service clicked:', item)}
              />
            </div>
          </>
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// TRACES VIEW - Using only core components
// ============================================================================

export const UnifiedTracesView = ({ traces }: any) => {
  const traceColumns = [
    {
      key: 'id',
      label: 'Trace ID',
      width: '200px',
      render: (item: any) => (
        <span style={{ fontFamily: 'monospace', fontSize: '11px', color: COLORS.accent.primary }}>
          {item.id?.slice(0, 16)}...
        </span>
      )
    },
    {
      key: 'service',
      label: 'Service',
      render: (item: any) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <StatusDot status={item.error ? 'error' : 'success'} />
          <span>{item.service}</span>
        </div>
      )
    },
    {
      key: 'operation',
      label: 'Operation',
      render: (item: any) => <Badge variant="primary">{item.operation}</Badge>
    },
    {
      key: 'duration',
      label: 'Duration',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: item.duration > 1000 ? COLORS.accent.warning : COLORS.text.secondary }}>
          {item.duration}ms
        </span>
      )
    },
    {
      key: 'spans',
      label: 'Spans',
      align: 'center' as const,
      render: (item: any) => item.spans
    },
    {
      key: 'time',
      label: 'Time',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: COLORS.text.tertiary }}>
          {new Date(item.time).toLocaleTimeString()}
        </span>
      )
    }
  ];

  return (
    <Page>
      <PageHeader
        title="Traces"
        subtitle="Distributed trace explorer"
        actions={<></>}
        metrics={
          <>
            <Metric label="Total Traces" value={traces?.length || 0} />
            <Metric label="Errors" value={traces?.filter((t: any) => t.has_error).length || 0} color="error" />
            <Metric label="Avg Duration" value={traces?.length ? `${Math.round(traces.reduce((acc: number, t: any) => acc + (t.duration?.as_millis || 0), 0) / traces.length)}ms` : '0ms'} />
            <Metric label="Spans/Trace" value={traces?.length ? (traces.reduce((acc: number, t: any) => acc + (t.span_count || 0), 0) / traces.length).toFixed(1) : '0'} />
          </>
        }
      />

      <div className="urpo-content">
        {!traces || traces.length === 0 ? (
          <EmptyState
            message="No traces found"
            description="Start sending OTLP data to see traces here"
          />
        ) : (
          <Table
            data={traces}
            columns={traceColumns}
            onRowClick={(item) => console.log('Trace clicked:', item)}
          />
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// SERVICES LIST VIEW - Using only core components
// ============================================================================

export const UnifiedServicesView = ({ services }: any) => {
  const servicesList = services || [];

  return (
    <Page>
      <PageHeader
        title="Services"
        subtitle={`${servicesList.length} services monitored`}
        actions={<></>}
      />

      <div className="urpo-content">
        {servicesList.length === 0 ? (
          <EmptyState
            message="No services detected"
            description="Start sending OTLP data to see services here"
          />
        ) : (
          <Grid cols={2} gap="md">
            {servicesList.map((service: any) => (
              <Card key={service.name}>
                <ListItem
                title={service.name}
                subtitle={`${service.trace_count || 0} traces • ${service.error_rate ? (service.error_rate * 100).toFixed(2) : 0}% errors`}
                value={service.avg_duration ? `${service.avg_duration.toFixed(0)}ms` : '0ms'}
                status={service.error_rate > 0.05 ? 'error' : 'success'}
                onClick={() => console.log('Service:', service)}
              />
              <div className="urpo-divider" />
              <Grid cols={3} gap="sm">
                <Metric label="Latency" value={service.avg_duration ? `${service.avg_duration.toFixed(0)}ms` : '0ms'} />
                <Metric label="Errors" value={`${service.error_rate ? (service.error_rate * 100).toFixed(2) : 0}%`} color={service.error_rate > 0.01 ? 'error' : undefined} />
                <Metric label="Traces" value={service.trace_count || 0} />
              </Grid>
            </Card>
          ))}
        </Grid>
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// DASHBOARD VIEW - Using only core components
// ============================================================================

export const UnifiedDashboardView = ({ data }: any) => {
  const services = data?.services || [];
  const traces = data?.traces || [];

  // Calculate real metrics from data
  const totalErrors = traces.filter((t: any) => t.has_error).length;
  const errorRate = traces.length > 0 ? ((totalErrors / traces.length) * 100).toFixed(2) : '0.00';
  const avgDuration = traces.length > 0
    ? Math.round(traces.reduce((acc: number, t: any) => acc + (t.duration?.as_millis || 0), 0) / traces.length)
    : 0;
  const p95Duration = traces.length > 0
    ? Math.round(traces.map((t: any) => t.duration?.as_millis || 0).sort((a: number, b: number) => b - a)[Math.floor(traces.length * 0.05)] || 0)
    : 0;

  const hasData = services.length > 0 || traces.length > 0;

  return (
    <Page>
      <PageHeader
        title="Dashboard"
        subtitle="System overview and key metrics"
        actions={<></>}
      />

      <div className="urpo-content">
        {!hasData ? (
          <EmptyState
            message="No data available"
            description="Start the OTLP receiver and send trace data to populate the dashboard"
          />
        ) : (
          <>
            {/* Key Metrics */}
            <Grid cols={4} gap="md">
              <Card>
                <Metric label="Services" value={services.length} color="primary" />
                <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
                  Active services
                </div>
              </Card>
              <Card>
                <Metric label="Total Traces" value={traces.length} trend={traces.length > 100 ? "up" : undefined} />
                <div style={{ fontSize: '11px', color: COLORS.accent.success, marginTop: '4px' }}>
                  Recent traces
                </div>
              </Card>
              <Card>
                <Metric
                  label="Error Rate"
                  value={`${errorRate}%`}
                  trend={parseFloat(errorRate) < 1 ? "down" : undefined}
                  color={parseFloat(errorRate) < 1 ? "success" : "error"}
                />
                <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
                  {parseFloat(errorRate) < 1 ? 'Within SLA' : 'Above threshold'}
                </div>
              </Card>
              <Card>
                <Metric label="P95 Latency" value={p95Duration > 0 ? `${p95Duration}ms` : 'N/A'} />
                <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
                  {avgDuration > 0 ? `Avg: ${avgDuration}ms` : 'No data'}
                </div>
              </Card>
            </Grid>

            {/* Recent Activity - Only show if there are traces */}
            {traces.length > 0 && (
              <div className="urpo-section">
                <h2 style={{ fontSize: '14px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
                  Recent Activity
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {traces.slice(0, 5).map((trace: any, idx: number) => (
                    <ListItem
                      key={idx}
                      title={trace.root_operation || 'Unknown operation'}
                      subtitle={`${trace.root_service || 'Unknown service'} • ${trace.span_count || 0} spans • ${trace.duration?.as_millis || 0}ms`}
                      status={trace.has_error ? 'error' : 'success'}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* System Status */}
            <Grid cols={3} gap="md">
              <Card>
                <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
                  Collection Stats
                </h3>
                <Grid cols={2} gap="sm">
                  <Metric label="Services" value={services.length} />
                  <Metric label="Traces" value={traces.length} />
                  <Metric label="Errors" value={totalErrors} color={totalErrors > 0 ? "error" : undefined} />
                  <Metric label="Avg Spans" value={traces.length > 0 ? (traces.reduce((acc: number, t: any) => acc + (t.span_count || 0), 0) / traces.length).toFixed(1) : '0'} />
                </Grid>
              </Card>

              <Card>
                <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
                  Performance
                </h3>
                <Grid cols={2} gap="sm">
                  <Metric label="Avg" value={avgDuration > 0 ? `${avgDuration}ms` : 'N/A'} />
                  <Metric label="P95" value={p95Duration > 0 ? `${p95Duration}ms` : 'N/A'} />
                  <Metric label="Fastest" value={traces.length > 0 ? `${Math.min(...traces.map((t: any) => t.duration?.as_millis || 0))}ms` : 'N/A'} />
                  <Metric label="Slowest" value={traces.length > 0 ? `${Math.max(...traces.map((t: any) => t.duration?.as_millis || 0))}ms` : 'N/A'} />
                </Grid>
              </Card>

              <Card>
                <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
                  Receivers
                </h3>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>OTLP/gRPC</span>
                    <Badge variant="success">Port 4327</Badge>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>OTLP/HTTP</span>
                    <Badge variant="success">Port 4328</Badge>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>Status</span>
                    <Badge variant={traces.length > 0 ? "success" : undefined}>
                      {traces.length > 0 ? 'Receiving' : 'Idle'}
                    </Badge>
                  </div>
                </div>
              </Card>
            </Grid>
          </>
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// METRICS VIEW - Using only core components
// ============================================================================

export const UnifiedMetricsView = () => {
  const [metrics, setMetrics] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const data = await invoke<any[]>('get_service_health_metrics');
        setMetrics(data || []);
      } catch (error) {
        console.error('Failed to fetch metrics:', error);
        setMetrics([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchMetrics();
    const interval = setInterval(fetchMetrics, 5000); // Refresh every 5s
    return () => clearInterval(interval);
  }, []);

  const metricsColumns = [
    {
      key: 'service_name',
      label: 'Service',
      render: (item: any) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <StatusDot status={item.error_rate > 5 ? 'error' : item.error_rate > 1 ? 'warning' : 'success'} />
          <span style={{ fontFamily: 'monospace', fontSize: '11px' }}>{item.service_name}</span>
        </div>
      )
    },
    {
      key: 'request_rate',
      label: 'Requests/sec',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: COLORS.text.secondary }}>
          {item.request_rate?.toFixed(2) || '0.00'}
        </span>
      )
    },
    {
      key: 'error_rate',
      label: 'Error %',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: item.error_rate > 5 ? COLORS.accent.error : COLORS.text.secondary }}>
          {item.error_rate?.toFixed(2) || '0.00'}%
        </span>
      )
    },
    {
      key: 'avg_latency_ms',
      label: 'Avg Latency',
      align: 'right' as const,
      render: (item: any) => `${item.avg_latency_ms?.toFixed(0) || '0'}ms`
    },
    {
      key: 'p95_latency_ms',
      label: 'P95 Latency',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: COLORS.text.tertiary }}>
          {item.p95_latency_ms?.toFixed(0) || '0'}ms
        </span>
      )
    }
  ];

  // Calculate summary stats
  const avgRequestRate = metrics.length > 0
    ? metrics.reduce((acc, m) => acc + (m.request_rate || 0), 0) / metrics.length
    : 0;
  const avgErrorRate = metrics.length > 0
    ? metrics.reduce((acc, m) => acc + (m.error_rate || 0), 0) / metrics.length
    : 0;
  const avgLatency = metrics.length > 0
    ? metrics.reduce((acc, m) => acc + (m.avg_latency_ms || 0), 0) / metrics.length
    : 0;

  return (
    <Page>
      <PageHeader
        title="Metrics"
        subtitle="Real-time OTLP metrics from services"
        actions={<></>}
        metrics={
          <>
            <Metric label="Services" value={metrics.length} />
            <Metric label="Avg Req/s" value={avgRequestRate.toFixed(2)} />
            <Metric
              label="Avg Error %"
              value={`${avgErrorRate.toFixed(2)}%`}
              color={avgErrorRate > 5 ? 'error' : avgErrorRate > 1 ? 'warning' : 'success'}
            />
            <Metric label="Avg Latency" value={avgLatency > 0 ? `${avgLatency.toFixed(0)}ms` : 'N/A'} />
          </>
        }
      />

      <div className="urpo-content">
        {isLoading ? (
          <LoadingState message="Loading metrics..." />
        ) : metrics.length === 0 ? (
          <EmptyState
            message="No metrics data"
            description="Start sending OTLP metrics to see real-time service health"
          />
        ) : (
          <>
            <Grid cols={4} gap="md">
              {metrics.slice(0, 4).map((metric) => (
                <Card key={metric.service_name}>
                  <div style={{ marginBottom: '12px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '4px' }}>
                      <StatusDot status={metric.error_rate > 5 ? 'error' : metric.error_rate > 1 ? 'warning' : 'success'} />
                      <span style={{ fontSize: '12px', fontWeight: 600, color: COLORS.text.primary }}>
                        {metric.service_name}
                      </span>
                    </div>
                  </div>
                  <Grid cols={2} gap="sm">
                    <Metric label="Req/s" value={metric.request_rate?.toFixed(1) || '0'} />
                    <Metric
                      label="Errors"
                      value={`${metric.error_rate?.toFixed(1) || '0'}%`}
                      color={metric.error_rate > 5 ? 'error' : undefined}
                    />
                    <Metric label="Latency" value={`${metric.avg_latency_ms?.toFixed(0) || '0'}ms`} />
                    <Metric label="P95" value={`${metric.p95_latency_ms?.toFixed(0) || '0'}ms`} />
                  </Grid>
                </Card>
              ))}
            </Grid>

            <div className="urpo-section">
              <Table
                data={metrics}
                columns={metricsColumns}
                onRowClick={(item) => console.log('Metric clicked:', item)}
              />
            </div>
          </>
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// LOGS VIEW - Using only core components
// ============================================================================

export const UnifiedLogsView = () => {
  const [logs, setLogs] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [severityFilter, setSeverityFilter] = useState<string>('');

  const fetchLogs = async () => {
    try {
      setIsLoading(true);
      let data;
      if (searchQuery) {
        data = await invoke<any[]>('search_logs', { query: searchQuery, limit: 1000 });
      } else {
        data = await invoke<any[]>('get_recent_logs', {
          limit: 1000,
          severityFilter: severityFilter || null
        });
      }
      setLogs(data || []);
    } catch (error) {
      console.error('Failed to fetch logs:', error);
      setLogs([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, 5000); // Refresh every 5s
    return () => clearInterval(interval);
  }, [searchQuery, severityFilter]);

  const getSeverityColor = (severity: string) => {
    switch (severity?.toUpperCase()) {
      case 'FATAL':
      case 'ERROR':
        return COLORS.accent.error;
      case 'WARN':
        return COLORS.accent.warning;
      case 'INFO':
        return COLORS.accent.primary;
      case 'DEBUG':
        return COLORS.text.secondary;
      case 'TRACE':
        return COLORS.text.tertiary;
      default:
        return COLORS.text.secondary;
    }
  };

  const getSeverityStatus = (severity: string): 'success' | 'warning' | 'error' | 'neutral' => {
    switch (severity?.toUpperCase()) {
      case 'FATAL':
      case 'ERROR':
        return 'error';
      case 'WARN':
        return 'warning';
      default:
        return 'neutral';
    }
  };

  const logsColumns = [
    {
      key: 'timestamp',
      label: 'Time',
      width: '140px',
      render: (item: any) => (
        <span style={{ fontSize: '10px', color: COLORS.text.tertiary, fontFamily: 'monospace' }}>
          {new Date(item.timestamp / 1000000).toLocaleString()}
        </span>
      )
    },
    {
      key: 'severity',
      label: 'Level',
      width: '80px',
      render: (item: any) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <StatusDot status={getSeverityStatus(item.severity)} />
          <Badge
            variant={item.severity === 'ERROR' || item.severity === 'FATAL' ? 'error' : undefined}
            style={{ fontSize: '10px' }}
          >
            {item.severity || 'INFO'}
          </Badge>
        </div>
      )
    },
    {
      key: 'body',
      label: 'Message',
      render: (item: any) => (
        <span style={{
          fontSize: '11px',
          fontFamily: 'monospace',
          color: getSeverityColor(item.severity)
        }}>
          {item.body?.substring(0, 200)}{item.body?.length > 200 ? '...' : ''}
        </span>
      )
    },
    {
      key: 'trace_id',
      label: 'Trace ID',
      width: '120px',
      render: (item: any) => item.trace_id ? (
        <span style={{ fontSize: '10px', fontFamily: 'monospace', color: COLORS.accent.primary }}>
          {item.trace_id.substring(0, 16)}...
        </span>
      ) : (
        <span style={{ fontSize: '10px', color: COLORS.text.tertiary }}>-</span>
      )
    }
  ];

  // Calculate severity counts
  const severityCounts = logs.reduce((acc, log) => {
    const severity = log.severity || 'INFO';
    acc[severity] = (acc[severity] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  const errorCount = (severityCounts['ERROR'] || 0) + (severityCounts['FATAL'] || 0);
  const warnCount = severityCounts['WARN'] || 0;

  return (
    <Page>
      <PageHeader
        title="Logs"
        subtitle="Real-time OTLP logs with trace correlation"
        actions={
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <Input
              value={searchQuery}
              onChange={setSearchQuery}
              placeholder="Search logs..."
              style={{ width: '200px', fontSize: '11px' }}
            />
            <select
              value={severityFilter}
              onChange={(e) => setSeverityFilter(e.target.value)}
              style={{
                padding: '6px 10px',
                background: COLORS.bg.elevated,
                color: COLORS.text.primary,
                border: `1px solid ${COLORS.border.subtle}`,
                borderRadius: '4px',
                fontSize: '11px',
                cursor: 'pointer'
              }}
            >
              <option value="">All Levels</option>
              <option value="FATAL">Fatal</option>
              <option value="ERROR">Error</option>
              <option value="WARN">Warn</option>
              <option value="INFO">Info</option>
              <option value="DEBUG">Debug</option>
              <option value="TRACE">Trace</option>
            </select>
          </div>
        }
        metrics={
          <>
            <Metric label="Total Logs" value={logs.length} />
            <Metric label="Errors" value={errorCount} color={errorCount > 0 ? 'error' : undefined} />
            <Metric label="Warnings" value={warnCount} color={warnCount > 0 ? 'warning' : undefined} />
            <Metric label="With Traces" value={logs.filter(l => l.trace_id).length} />
          </>
        }
      />

      <div className="urpo-content">
        {isLoading ? (
          <LoadingState message="Loading logs..." />
        ) : logs.length === 0 ? (
          <EmptyState
            message="No logs found"
            description="Start sending OTLP logs to see them here"
          />
        ) : (
          <div className="urpo-section">
            <Table
              data={logs}
              columns={logsColumns}
              onRowClick={(item) => {
                console.log('Log clicked:', item);
                if (item.trace_id) {
                  // TODO: Navigate to trace view with this trace_id
                  console.log('Navigate to trace:', item.trace_id);
                }
              }}
            />
          </div>
        )}
      </div>
    </Page>
  );
};