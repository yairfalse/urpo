import React from 'react';

function TestApp() {
    return (
        <div style={{ padding: '20px', fontFamily: 'monospace', background: '#1a1a1a', color: '#fff', minHeight: '100vh' }}>
            <h1>🚀 Urpo Test App</h1>
            <p>If you can see this, React is working!</p>
            <button onClick={() => alert('Clicked!')}>Test Button</button>
            <pre>{JSON.stringify({
                time: new Date().toISOString(),
                react: '✅ Working',
                render: '✅ Success'
            }, null, 2)}</pre>
        </div>
    );
}

export default TestApp;