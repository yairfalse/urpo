/**
 * Unified Views - All pages using ONLY core design system
 * This is how every page should be built
 */

import React from 'react';
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