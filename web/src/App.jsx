import { useEffect, useMemo, useState } from 'react';
import './App.css';

const SECTION_TITLES = {
  summary: 'Summary',
  metrics: 'Metrics',
  issues: 'Issues',
  logs: 'Logs',
  metadata: 'Metadata'
};

function formatKey(key) {
  if (SECTION_TITLES[key]) {
    return SECTION_TITLES[key];
  }

  return key
    .replace(/([A-Z])/g, ' $1')
    .replace(/[-_]/g, ' ')
    .replace(/^./, (str) => str.toUpperCase());
}

function ValueRenderer({ value }) {
  if (value === null || value === undefined || value === '') {
    return <span className="muted">N/A</span>;
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return <span className="muted">No entries</span>;
    }

    return (
      <ul className="list">
        {value.map((item, index) => (
          <li key={index}>
            <ValueRenderer value={item} />
          </li>
        ))}
      </ul>
    );
  }

  if (typeof value === 'object') {
    return (
      <dl className="key-value">
        {Object.entries(value).map(([childKey, childValue]) => (
          <div className="row" key={childKey}>
            <dt>{formatKey(childKey)}</dt>
            <dd>
              <ValueRenderer value={childValue} />
            </dd>
          </div>
        ))}
      </dl>
    );
  }

  if (typeof value === 'boolean') {
    return <span>{value ? 'Yes' : 'No'}</span>;
  }

  if (typeof value === 'number') {
    return <span>{value.toLocaleString()}</span>;
  }

  return <span>{String(value)}</span>;
}

export default function App() {
  const [data, setData] = useState(null);
  const [error, setError] = useState(null);
  const [status, setStatus] = useState('idle');

  useEffect(() => {
    let isMounted = true;

    async function load() {
      try {
        setStatus('loading');
        const response = await fetch('/data/output.json');

        if (!response.ok) {
          throw new Error(`Request failed with status ${response.status}`);
        }

        const payload = await response.json();
        if (isMounted) {
          setData(payload);
          setError(null);
          setStatus('success');
        }
      } catch (err) {
        if (isMounted) {
          setError(err);
          setStatus('error');
        }
      }
    }

    load();

    const interval = setInterval(load, 60_000);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, []);

  const sections = useMemo(() => {
    if (!data || typeof data !== 'object') {
      return [];
    }

    return Object.entries(data).map(([key, value]) => ({
      key,
      title: formatKey(key),
      value
    }));
  }, [data]);

  return (
    <div className="app">
      <header className="app__header">
        <h1>Elastic Debugger Report</h1>
        <p className="muted">
          The latest information from <code>/data/output.json</code> is displayed below. The
          view automatically refreshes every minute.
        </p>
      </header>

      {status === 'loading' && (
        <section className="card placeholder">
          <div className="skeleton skeleton--title" />
          <div className="skeleton skeleton--line" />
          <div className="skeleton skeleton--line" />
        </section>
      )}

      {status === 'error' && (
        <section className="card error">
          <h2>Unable to load data</h2>
          <p>
            The dashboard could not retrieve <code>/data/output.json</code>. The file might be
            missing or the server may be unreachable. The view will keep retrying automatically.
          </p>
          <pre className="error__details">{error?.message}</pre>
        </section>
      )}

      {status === 'success' && sections.length === 0 && (
        <section className="card">
          <h2>No data available</h2>
          <p>The data file was empty. Once content is added it will appear here automatically.</p>
        </section>
      )}

      <div className="grid">
        {sections.map((section) => (
          <section className="card" key={section.key} aria-label={section.title}>
            <header className="card__header">
              <h2>{section.title}</h2>
            </header>
            <div className="card__body">
              <ValueRenderer value={section.value} />
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
