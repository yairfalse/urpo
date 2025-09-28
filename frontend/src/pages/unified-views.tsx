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
  const healthColumns = [
    {
      key: 'name',
      label: 'Service',
      render: (item: any) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <StatusDot status={item.status} />
          <span>{item.name}</span>
        </div>
      )
    },
    {
      key: 'requests',
      label: 'Requests/s',
      align: 'right' as const,
      render: (item: any) => <Badge variant={item.requests > 100 ? 'warning' : 'success'}>{item.requests}</Badge>
    },
    {
      key: 'errors',
      label: 'Errors',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: item.errors > 0 ? COLORS.accent.error : COLORS.text.tertiary }}>
          {item.errors}%
        </span>
      )
    },
    {
      key: 'latency',
      label: 'P95 Latency',
      align: 'right' as const,
      render: (item: any) => `${item.latency}ms`
    },
    {
      key: 'uptime',
      label: 'Uptime',
      align: 'right' as const,
      render: (item: any) => (
        <span style={{ color: COLORS.text.secondary }}>{item.uptime}</span>
      )
    }
  ];

  return (
    <Page>
      <PageHeader
        title="Service Health"
        subtitle="Real-time service metrics and health status"
        actions={<></>}
        metrics={
          <>
            <Metric label="Total Services" value={services?.length || 0} />
            <Metric label="Healthy" value={services?.filter((s: any) => !s.error_rate || s.error_rate < 0.01).length || 0} color="success" />
            <Metric label="Warning" value={services?.filter((s: any) => s.error_rate >= 0.01 && s.error_rate < 0.05).length || 0} color="warning" />
            <Metric label="Critical" value={services?.filter((s: any) => s.error_rate >= 0.05).length || 0} color="error" />
            <Metric label="Avg Response" value={services?.length ? `${Math.round(services.reduce((acc: number, s: any) => acc + (s.avg_duration || 0), 0) / services.length)}ms` : '0ms'} />
          </>
        }
      />

      <div className="urpo-content">
        <Grid cols={4} gap="md">
          <Card>
            <Metric label="Total Requests" value="1.2M" trend="up" />
          </Card>
          <Card>
            <Metric label="Error Rate" value="0.02%" trend="down" color="success" />
          </Card>
          <Card>
            <Metric label="Avg Latency" value="89ms" trend="neutral" />
          </Card>
          <Card>
            <Metric label="Active Traces" value="423" color="primary" />
          </Card>
        </Grid>

        <div className="urpo-section">
          <Table
            data={services || []}
            columns={healthColumns}
            onRowClick={(item) => console.log('Service clicked:', item)}
          />
        </div>
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

  return (
    <Page>
      <PageHeader
        title="Dashboard"
        subtitle="System overview and key metrics"
        actions={<></>}
      />

      <div className="urpo-content">
        {/* Key Metrics */}
        <Grid cols={4} gap="md">
          <Card>
            <Metric label="Services" value={services.length} color="primary" />
            <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
              Active services
            </div>
          </Card>
          <Card>
            <Metric label="Total Traces" value={traces.length} trend="up" />
            <div style={{ fontSize: '11px', color: COLORS.accent.success, marginTop: '4px' }}>
              Recent traces
            </div>
          </Card>
          <Card>
            <Metric label="Error Rate" value="0.08%" trend="down" color="success" />
            <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
              Well within SLA
            </div>
          </Card>
          <Card>
            <Metric label="P95 Latency" value="234ms" />
            <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
              Stable performance
            </div>
          </Card>
        </Grid>

        {/* Recent Activity */}
        <div className="urpo-section">
          <h2 style={{ fontSize: '14px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
            Recent Activity
          </h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <ListItem
              title="High latency detected"
              subtitle="checkout-service • 2 minutes ago"
              status="warning"
            />
            <ListItem
              title="New service deployed"
              subtitle="payment-gateway • 15 minutes ago"
              status="success"
            />
            <ListItem
              title="Error spike resolved"
              subtitle="auth-service • 1 hour ago"
              status="info"
            />
          </div>
        </div>

        {/* System Status */}
        <Grid cols={3} gap="md">
          <Card>
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
              Infrastructure
            </h3>
            <Grid cols={2} gap="sm">
              <Metric label="CPU" value="42%" />
              <Metric label="Memory" value="67%" color="warning" />
              <Metric label="Disk" value="31%" />
              <Metric label="Network" value="12Mb/s" />
            </Grid>
          </Card>

          <Card>
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
              Storage
            </h3>
            <Grid cols={2} gap="sm">
              <Metric label="Traces" value="1.2GB" />
              <Metric label="Spans" value="423K" />
              <Metric label="Retention" value="7 days" />
              <Metric label="Compression" value="82%" color="success" />
            </Grid>
          </Card>

          <Card>
            <h3 style={{ fontSize: '13px', fontWeight: 600, color: COLORS.text.primary, marginBottom: '12px' }}>
              Receivers
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>OTLP/gRPC</span>
                <Badge variant="success">Active</Badge>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>OTLP/HTTP</span>
                <Badge variant="success">Active</Badge>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span style={{ fontSize: '12px', color: COLORS.text.secondary }}>Jaeger</span>
                <Badge>Idle</Badge>
              </div>
            </div>
          </Card>
        </Grid>
      </div>
    </Page>
  );
};