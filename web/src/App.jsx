import { useEffect, useMemo, useState } from 'react';
import chainIdsDoc from './chain_ids.json';
import './App.css';

// Helpers
const shorten = (v) => {
  if (!v || typeof v !== 'string') return String(v ?? '');
  if (!v.startsWith('0x') || v.length <= 12) return v;
  return `${v.slice(0, 8)}…${v.slice(-6)}`;
};

const formatVersion = (ver) => {
  if (!ver) return 'n/a';
  const [a, b, c] = ver;
  return `${a}.${b}.${c}`;
};

function Badge({ tone = 'neutral', children }) {
  return <span className={`badge badge--${tone}`}>{children}</span>;
}

function Collapsible({ title, defaultOpen = false, children, count }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="collapsible">
      <button className="collapsible__trigger" onClick={() => setOpen((v) => !v)}>
        <span className={`chevron ${open ? 'open' : ''}`}>▸</span>
        <span>{title}</span>
        {typeof count === 'number' && <span className="muted">({count})</span>}
      </button>
      {open && <div className="collapsible__content">{children}</div>}
    </div>
  );
}

function NodeBox({ title, subtitle, status, details, arrowTo }) {
  return (
    <section className="node card">
      <header className="node__header">
        <h2 className="node__title">{title}</h2>
        {subtitle && <div className="node__subtitle">{subtitle}</div>}
        {status && <Badge tone={status === 'ok' ? 'success' : 'danger'}>{status.toUpperCase()}</Badge>}
      </header>
      {arrowTo && (
        <div className="arrow">
          <span className="arrow__label">settles to</span>
          <span className="arrow__icon">→</span>
          <span className="arrow__target">{arrowTo}</span>
        </div>
      )}
      {details}
    </section>
  );
}

function KeyValue({ label, value }) {
  return (
    <div className="kv">
      <div className="kv__k">{label}</div>
      <div className="kv__v">{value ?? <span className="muted">N/A</span>}</div>
    </div>
  );
}

function PriorityTable({ txs }) {
  if (!txs || txs.length === 0) {
    return <div className="muted">No priority transactions</div>;
  }
  return (
    <div className="table-wrapper">
      <table className="table">
        <thead>
          <tr>
            <th>#</th>
            <th>Method</th>
            <th>From</th>
            <th>To</th>
            <th>Value</th>
            <th>Gas</th>
          </tr>
        </thead>
        <tbody>
          {txs.map((t) => (
            <tr key={`${t.index}-${t.tx_id}`}>
              <td>{t.index}</td>
              <td>{t.method ?? 'unknown'}</td>
              <td><code className="mono">{t.from}</code></td>
              <td><code className="mono">{t.to}</code></td>
              <td>{t.value_formatted}</td>
              <td>{t.gas_limit}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ChainCard({ chain, settlesTo }) {
  const st = chain.state_transition;
  const ok = Boolean(st);
  return (
    <section className="chain card">
      <header className="chain__header">
        <div className="row-left">
          <h3 className="chain__title">Chain {chain.chain_id}</h3>
          <span className="settlement">→ {settlesTo}</span>
        </div>
        <div className="row-right">
          <Badge tone={ok ? 'success' : 'danger'}>{ok ? 'HEALTHY' : 'ERROR'}</Badge>
        </div>
      </header>
      {ok ? (
        <div className="stats">
          <div className="pill">Protocol {formatVersion(st.protocol_version)}</div>
          <div className="pill">Batches C/V/E {st.total_batches_committed}/{st.total_batches_verified}/{st.total_batches_executed}</div>
          <div className="pill">Queue {st.queue.unprocessed}/{st.queue.total}</div>
          {typeof chain.priority_tree_verified === 'boolean' && (
            <div className={`pill ${chain.priority_tree_verified ? 'pill--ok' : 'pill--warn'}`}>
              Priority root {chain.priority_tree_verified ? 'VALID' : 'INVALID'}
            </div>
          )}
        </div>
      ) : (
        <div className="muted">{chain.state_transition_error ?? 'State transition unavailable'}</div>
      )}

      {ok && (
        <div className="grid-2">
          <KeyValue label="Hyperchain" value={<code className="mono">{shorten(st.hyperchain)}</code>} />
          <KeyValue label="Verifier" value={<code className="mono">{shorten(st.verifier)}</code>} />
          <KeyValue label="Admin" value={<code className="mono">{shorten(st.admin)}</code>} />
          <KeyValue label="Settlement layer" value={<code className="mono">{shorten(st.settlement_layer)}</code>} />
        </div>
      )}

      <Collapsible title="Priority transactions" count={chain.priority_transactions?.length || 0}>
        <PriorityTable txs={chain.priority_transactions} />
      </Collapsible>
    </section>
  );
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
        const response = await fetch('output.json');
        if (!response.ok) throw new Error(`Request failed with status ${response.status}`);
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

  // Build id -> name map from src/chain_ids.json (which maps name -> id)
  const idToName = useMemo(() => {
    try {
      const mapping = chainIdsDoc?.chain_ids || {};
      const inv = new Map();
      for (const [name, id] of Object.entries(mapping)) {
        inv.set(Number(id), name);
      }
      return inv;
    } catch {
      return new Map();
    }
  }, []);

  const gatewayChainSet = useMemo(() => {
    const s = new Set();
    if (data?.gateway_bridgehub?.known_chains) {
      for (const id of data.gateway_bridgehub.known_chains) s.add(Number(id));
    }
    return s;
  }, [data]);

  // Determine newest and previous protocol versions across all chains
  const versionRanks = useMemo(() => {
    if (!data?.chains) return { latest: null, previous: null, asKey: () => '' };
    const toKey = (v) => (Array.isArray(v) && v.length === 3 ? `${v[0]}.${v[1]}.${v[2]}` : '');
    const fromKey = (k) => k.split('.').map((n) => Number(n));
    const uniq = new Set();
    for (const c of data.chains) {
      const v = c?.state_transition?.protocol_version;
      const key = toKey(v);
      if (key) uniq.add(key);
    }
    const list = Array.from(uniq);
    list.sort((a, b) => {
      const [ax, ay, az] = fromKey(a);
      const [bx, by, bz] = fromKey(b);
      if (ax !== bx) return ax - bx;
      if (ay !== by) return ay - by;
      return az - bz;
    });
    const latest = list[list.length - 1] || null;
    const previous = list.length > 1 ? list[list.length - 2] : null;
    return { latest, previous, asKey: toKey };
  }, [data]);

  const chainsBySettlement = useMemo(() => {
    if (!data?.chains) return { toGateway: [], toL1: [] };
    const toGateway = [];
    const toL1 = [];
    for (const c of data.chains) {
      if (gatewayChainSet.has(Number(c.chain_id))) toGateway.push(c);
      else toL1.push(c);
    }
    return { toGateway, toL1 };
  }, [data, gatewayChainSet]);

  return (
    <div className="app">
      <header className="app__header">
        <h1>Elastic Debugger Report</h1>
        <p className="muted">
          Loaded from <code>/data/output.json</code>. Auto-refreshes every minute.
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

      {status === 'success' && (
        <div className="layout">
          <div className="layout__row">
            {/* L1 */}
            <NodeBox
              title="L1"
              subtitle={data?.sequencers?.l1?.sequencer?.rpc_url}
              status={data?.sequencers?.l1?.status}
              details={
                <div className="grid-2">
                  <KeyValue label="Chain ID" value={data?.sequencers?.l1?.sequencer?.chain_id} />
                  <KeyValue label="Latest block" value={data?.sequencers?.l1?.sequencer?.latest_block} />
                  <KeyValue label="Bridgehub" value={<code className="mono">{shorten(data?.bridgehub?.address)}</code>} />
                  <KeyValue label="CTM deployer" value={<code className="mono">{shorten(data?.bridgehub?.ctm_deployer)}</code>} />
                  <KeyValue label="Known chains" value={data?.bridgehub?.known_chains?.length ?? 0} />
                </div>
              }
            />

            {/* Gateway (if present) */}
            {data?.gateway_bridgehub && (
              <NodeBox
                title="Gateway"
                subtitle={data?.sequencers?.l2?.sequencer?.rpc_url}
                status={data?.sequencers?.l2?.status}
                arrowTo="L1"
                details={
                  <div className="grid-2">
                    <KeyValue label="Chain ID" value={data?.sequencers?.l2?.sequencer?.chain_id} />
                    <KeyValue label="Latest block" value={data?.sequencers?.l2?.sequencer?.latest_block} />
                    <KeyValue label="Bridgehub" value={<code className="mono">{shorten(data?.gateway_bridgehub?.address)}</code>} />
                    <KeyValue label="Known chains" value={data?.gateway_bridgehub?.known_chains?.length ?? 0} />
                  </div>
                }
              />
            )}
          </div>

          <div className="layout__row">
            <div className="column">
              <h2 className="section-title">Chains settling to Gateway</h2>
              <div className="grid">
                {chainsBySettlement.toGateway.map((c) => {
                  const st = c.state_transition;
                  const name = idToName.get(Number(c.chain_id));
                  const key = versionRanks.asKey(st?.protocol_version);
                  const verTone = key
                    ? key === versionRanks.latest
                      ? 'ok'
                      : key === versionRanks.previous
                      ? 'warn'
                      : 'danger'
                    : 'neutral';
                  return (
                    <section className="chain card" key={c.chain_id}>
                      <header className="chain__header">
                        <div className="row-left">
                          <h3 className="chain__title">Chain {c.chain_id}{name ? ` · ${name}` : ''}</h3>
                          <span className="settlement">→ Gateway</span>
                        </div>
                        <div className="row-right">
                          <Badge tone={st ? 'success' : 'danger'}>{st ? 'HEALTHY' : 'ERROR'}</Badge>
                        </div>
                      </header>
                      {st ? (
                        <>
                          <div className="stats">
                            <div className={`pill ${verTone === 'ok' ? 'pill--ok' : verTone === 'warn' ? 'pill--warn' : verTone === 'danger' ? 'pill--danger' : ''}`}>
                              Protocol {formatVersion(st.protocol_version)}
                            </div>
                            <div className="pill">Batches C/V/E {st.total_batches_committed}/{st.total_batches_verified}/{st.total_batches_executed}</div>
                            <div className="pill">Queue {st.queue.unprocessed}/{st.queue.total}</div>
                            {typeof c.priority_tree_verified === 'boolean' && (
                              <div className={`pill ${c.priority_tree_verified ? 'pill--ok' : 'pill--warn'}`}>
                                Priority root {c.priority_tree_verified ? 'VALID' : 'INVALID'}
                              </div>
                            )}
                          </div>
                          <div className="grid-2">
                            <KeyValue label="Hyperchain" value={<code className="mono">{shorten(st.hyperchain)}</code>} />
                            <KeyValue label="Verifier" value={<code className="mono">{shorten(st.verifier)}</code>} />
                            <KeyValue label="Admin" value={<code className="mono">{shorten(st.admin)}</code>} />
                            <KeyValue label="Settlement layer" value={<code className="mono">{shorten(st.settlement_layer)}</code>} />
                          </div>
                          <Collapsible title="Priority transactions" count={c.priority_transactions?.length || 0}>
                            <PriorityTable txs={c.priority_transactions} />
                          </Collapsible>
                        </>
                      ) : (
                        <div className="muted">{c.state_transition_error ?? 'State transition unavailable'}</div>
                      )}
                    </section>
                  );
                })}
                {chainsBySettlement.toGateway.length === 0 && (
                  <div className="muted">No chains registered on Gateway</div>
                )}
              </div>
            </div>

            <div className="column">
              <h2 className="section-title">Chains settling to L1</h2>
              <div className="grid">
                {chainsBySettlement.toL1.map((c) => {
                  const st = c.state_transition;
                  const name = idToName.get(Number(c.chain_id));
                  const key = versionRanks.asKey(st?.protocol_version);
                  const verTone = key
                    ? key === versionRanks.latest
                      ? 'ok'
                      : key === versionRanks.previous
                      ? 'warn'
                      : 'danger'
                    : 'neutral';
                  return (
                    <section className="chain card" key={c.chain_id}>
                      <header className="chain__header">
                        <div className="row-left">
                          <h3 className="chain__title">Chain {c.chain_id}{name ? ` · ${name}` : ''}</h3>
                          <span className="settlement">→ L1</span>
                        </div>
                        <div className="row-right">
                          <Badge tone={st ? 'success' : 'danger'}>{st ? 'HEALTHY' : 'ERROR'}</Badge>
                        </div>
                      </header>
                      {st ? (
                        <>
                          <div className="stats">
                            <div className={`pill ${verTone === 'ok' ? 'pill--ok' : verTone === 'warn' ? 'pill--warn' : verTone === 'danger' ? 'pill--danger' : ''}`}>
                              Protocol {formatVersion(st.protocol_version)}
                            </div>
                            <div className="pill">Batches C/V/E {st.total_batches_committed}/{st.total_batches_verified}/{st.total_batches_executed}</div>
                            <div className="pill">Queue {st.queue.unprocessed}/{st.queue.total}</div>
                            {typeof c.priority_tree_verified === 'boolean' && (
                              <div className={`pill ${c.priority_tree_verified ? 'pill--ok' : 'pill--warn'}`}>
                                Priority root {c.priority_tree_verified ? 'VALID' : 'INVALID'}
                              </div>
                            )}
                          </div>
                          <div className="grid-2">
                            <KeyValue label="Hyperchain" value={<code className="mono">{shorten(st.hyperchain)}</code>} />
                            <KeyValue label="Verifier" value={<code className="mono">{shorten(st.verifier)}</code>} />
                            <KeyValue label="Admin" value={<code className="mono">{shorten(st.admin)}</code>} />
                            <KeyValue label="Settlement layer" value={<code className="mono">{shorten(st.settlement_layer)}</code>} />
                          </div>
                          <Collapsible title="Priority transactions" count={c.priority_transactions?.length || 0}>
                            <PriorityTable txs={c.priority_transactions} />
                          </Collapsible>
                        </>
                      ) : (
                        <div className="muted">{c.state_transition_error ?? 'State transition unavailable'}</div>
                      )}
                    </section>
                  );
                })}
                {chainsBySettlement.toL1.length === 0 && (
                  <div className="muted">No chains registered on L1</div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
