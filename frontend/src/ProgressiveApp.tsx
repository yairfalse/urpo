import { useState, useEffect } from 'react';

// Test basic state management
function ProgressiveApp() {
  const [counter, setCounter] = useState(0);
  const [status, setStatus] = useState('Starting...');
  const [components, setComponents] = useState<string[]>([]);

  useEffect(() => {
    console.log('ProgressiveApp mounted');
    setStatus('React is working ✅');
  }, []);

  const testStateUpdate = () => {
    setCounter(c => c + 1);
    console.log('Button clicked, counter:', counter + 1);
  };

  const loadHooks = async () => {
    try {
      setStatus('Loading hooks...');
      const { useLocalStorage } = await import('./hooks/useLocalStorage');
      setComponents(prev => [...prev, '✅ useLocalStorage']);

      const { useKeyboardShortcuts } = await import('./hooks/useKeyboardShortcuts');
      setComponents(prev => [...prev, '✅ useKeyboardShortcuts']);

      setStatus('Hooks loaded successfully!');
    } catch (error: any) {
      setStatus(`❌ Hook Error: ${error.message}`);
      console.error('Hook loading error:', error);
    }
  };

  const loadComponents = async () => {
    try {
      setStatus('Loading components...');

      // Try loading ErrorBoundary first
      const { ErrorBoundary } = await import('./components/common/ErrorBoundary');
      setComponents(prev => [...prev, '✅ ErrorBoundary']);

      // Try loading a simple table component
      const { ServiceHealthDashboard } = await import('./components/tables/ServiceHealthDashboard');
      setComponents(prev => [...prev, '✅ ServiceHealthDashboard']);

      setStatus('Components loaded successfully!');
    } catch (error: any) {
      setStatus(`❌ Component Error: ${error.message}`);
      console.error('Component loading error:', error);
    }
  };

  const loadFullApp = async () => {
    try {
      setStatus('Loading full App...');
      const App = await import('./App');
      setStatus('✅ Full App loaded! Switching view...');

      // Replace this component with the full App
      const root = document.getElementById('root');
      if (root) {
        const ReactDOM = await import('react-dom/client');
        const rootElement = ReactDOM.createRoot(root);
        rootElement.render(<App.default />);
      }
    } catch (error: any) {
      setStatus(`❌ Full App Error: ${error.message}`);
      console.error('Full App loading error:', error);
    }
  };

  return (
    <div style={{
      padding: '20px',
      fontFamily: 'monospace',
      background: '#1a1a1a',
      color: '#fff',
      minHeight: '100vh'
    }}>
      <h1>🔧 Urpo Progressive Loader</h1>
      <p>Status: {status}</p>
      <p>Counter: {counter} (Click test button to verify state works)</p>

      <div style={{ marginTop: '20px', display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
        <button
          onClick={testStateUpdate}
          style={{
            padding: '10px 20px',
            background: '#007acc',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Test State (Counter: {counter})
        </button>

        <button
          onClick={loadHooks}
          style={{
            padding: '10px 20px',
            background: '#28a745',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Load Hooks
        </button>

        <button
          onClick={loadComponents}
          style={{
            padding: '10px 20px',
            background: '#ffc107',
            color: 'black',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Load Components
        </button>

        <button
          onClick={loadFullApp}
          style={{
            padding: '10px 20px',
            background: '#dc3545',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          Load Full App
        </button>
      </div>

      {components.length > 0 && (
        <div style={{
          marginTop: '20px',
          padding: '20px',
          background: '#2a2a2a',
          borderRadius: '5px'
        }}>
          <h3>Loaded Components:</h3>
          <ul>
            {components.map((comp, i) => (
              <li key={i}>{comp}</li>
            ))}
          </ul>
        </div>
      )}

      <div style={{
        marginTop: '20px',
        padding: '10px',
        background: '#2a2a2a',
        borderRadius: '5px',
        fontSize: '12px'
      }}>
        <p>Console output will show detailed errors</p>
        <p>Time: {new Date().toISOString()}</p>
      </div>
    </div>
  );
}

export default ProgressiveApp;