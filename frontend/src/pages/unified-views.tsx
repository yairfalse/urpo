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
        actions={
          <>
            <Button variant="ghost" size="sm">Filter</Button>
            <Button variant="secondary" size="sm">Export</Button>
            <Button variant="primary" size="sm">Configure Alerts</Button>
          </>
        }
        metrics={
          <>
            <Metric label="Total Services" value={services?.length || 0} />
            <Metric label="Healthy" value={12} color="success" />
            <Metric label="Warning" value={3} color="warning" />
            <Metric label="Critical" value={1} color="error" />
            <Metric label="Avg Response" value="124ms" />
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
        actions={
          <>
            <Input
              value=""
              onChange={() => {}}
              placeholder="Search traces..."
              className="urpo-search"
            />
            <Button variant="secondary" size="sm">Filters</Button>
            <Button variant="primary" size="sm">New Query</Button>
          </>
        }
        metrics={
          <>
            <Metric label="Total Traces" value={traces?.length || 0} />
            <Metric label="Errors" value={23} color="error" />
            <Metric label="Avg Duration" value="234ms" />
            <Metric label="Spans/Trace" value="14.2" />
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
  return (
    <Page>
      <PageHeader
        title="Services"
        subtitle="Service inventory and dependencies"
        actions={
          <>
            <Button variant="ghost" size="sm">Refresh</Button>
            <Button variant="primary" size="sm">Add Service</Button>
          </>
        }
      />

      <div className="urpo-content">
        <Grid cols={2} gap="md">
          {services?.map((service: any) => (
            <Card key={service.name}>
              <ListItem
                title={service.name}
                subtitle={`${service.endpoints} endpoints • ${service.instances} instances`}
                value={service.requests}
                status={service.health}
                onClick={() => console.log('Service:', service)}
              />
              <div className="urpo-divider" />
              <Grid cols={3} gap="sm">
                <Metric label="Latency" value={`${service.latency}ms`} />
                <Metric label="Errors" value={`${service.errors}%`} color={service.errors > 1 ? 'error' : undefined} />
                <Metric label="Uptime" value={service.uptime} />
              </Grid>
            </Card>
          ))}
        </Grid>

        {(!services || services.length === 0) && (
          <EmptyState
            message="No services discovered"
            description="Services will appear here once they start sending telemetry data"
          />
        )}
      </div>
    </Page>
  );
};

// ============================================================================
// DASHBOARD VIEW - Using only core components
// ============================================================================

export const UnifiedDashboardView = ({ data }: any) => {
  return (
    <Page>
      <PageHeader
        title="Dashboard"
        subtitle="System overview and key metrics"
        actions={
          <>
            <Button variant="ghost" size="sm">Last 15 min</Button>
            <Button variant="secondary" size="sm">Refresh</Button>
          </>
        }
      />

      <div className="urpo-content">
        {/* Key Metrics */}
        <Grid cols={4} gap="md">
          <Card>
            <Metric label="Services" value={16} color="primary" />
            <div style={{ fontSize: '11px', color: COLORS.text.tertiary, marginTop: '4px' }}>
              +2 from last hour
            </div>
          </Card>
          <Card>
            <Metric label="Total Traces" value="45.2K" trend="up" />
            <div style={{ fontSize: '11px', color: COLORS.accent.success, marginTop: '4px' }}>
              ↑ 12% from baseline
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