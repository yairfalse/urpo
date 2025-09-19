import { useState, useEffect } from 'react';

function AppDebug() {
  const [step, setStep] = useState(1);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    console.log(`AppDebug mounted - Step ${step}`);
  }, [step]);

  try {
    return (
      <div style={{
        padding: '20px',
        fontFamily: 'monospace',
        background: '#1a1a1a',
        color: '#fff',
        minHeight: '100vh'
      }}>
        <h1>🚀 Urpo Debug Mode</h1>
        <p>Testing components step by step...</p>

        <div style={{ marginTop: '20px' }}>
          <button onClick={() => setStep(1)} style={{ marginRight: '10px' }}>Step 1: Basic</button>
          <button onClick={() => setStep(2)} style={{ marginRight: '10px' }}>Step 2: Hooks</button>
          <button onClick={() => setStep(3)} style={{ marginRight: '10px' }}>Step 3: Components</button>
          <button onClick={() => setStep(4)} style={{ marginRight: '10px' }}>Step 4: Full App</button>
        </div>

        <div style={{
          marginTop: '20px',
          padding: '20px',
          border: '1px solid #444',
          borderRadius: '5px'
        }}>
          {step === 1 && (
            <div>
              <h2>✅ Step 1: Basic React Working</h2>
              <p>If you see this, React is rendering correctly.</p>
            </div>
          )}

          {step === 2 && (
            <div>
              <h2>Step 2: Testing Hooks...</h2>
              {(() => {
                try {
                  // Test if hooks work
                  const { useLocalStorage } = require('./hooks/useLocalStorage');
                  const [testValue] = useLocalStorage('test', 'default');
                  return <p>✅ Hooks working! Test value: {testValue}</p>;
                } catch (e: any) {
                  return <p style={{ color: '#ff6b6b' }}>❌ Hook Error: {e.message}</p>;
                }
              })()}
            </div>
          )}

          {step === 3 && (
            <div>
              <h2>Step 3: Testing Components...</h2>
              {(() => {
                try {
                  // Test if components can be imported
                  const { ErrorBoundary } = require('./components/common/ErrorBoundary');
                  return (
                    <ErrorBoundary>
                      <p>✅ Components can be imported!</p>
                    </ErrorBoundary>
                  );
                } catch (e: any) {
                  return <p style={{ color: '#ff6b6b' }}>❌ Component Error: {e.message}</p>;
                }
              })()}
            </div>
          )}

          {step === 4 && (
            <div>
              <h2>Step 4: Loading Full App...</h2>
              {(() => {
                try {
                  const App = require('./App').default;
                  return <App />;
                } catch (e: any) {
                  return (
                    <div>
                      <p style={{ color: '#ff6b6b' }}>❌ Full App Error:</p>
                      <pre style={{
                        background: '#2a2a2a',
                        padding: '10px',
                        borderRadius: '5px',
                        overflow: 'auto'
                      }}>
                        {e.stack || e.message}
                      </pre>
                    </div>
                  );
                }
              })()}
            </div>
          )}
        </div>

        {error && (
          <div style={{
            marginTop: '20px',
            padding: '20px',
            background: '#ff6b6b22',
            border: '1px solid #ff6b6b',
            borderRadius: '5px'
          }}>
            <h3>Error Details:</h3>
            <pre>{error}</pre>
          </div>
        )}
      </div>
    );
  } catch (e: any) {
    return (
      <div style={{ padding: '20px', color: '#ff6b6b' }}>
        <h1>Critical Error in AppDebug</h1>
        <pre>{e.stack || e.message}</pre>
      </div>
    );
  }
}

export default AppDebug;