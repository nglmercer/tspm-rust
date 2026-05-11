import { useState, useEffect, useCallback } from 'preact/hooks';
import { api } from '../api/client';
import type { PortInfo } from '../types';

export function PortsView() {
    const [ports, setPorts] = useState<PortInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    const fetch = useCallback(async () => {
        setLoading(true);
        try {
            const res = await api.ports.list();
            if (res.success) setPorts(res.data ?? []);
        } catch (e: any) {
            setError(e.message);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => { fetch(); }, [fetch]);

    const kill = async (port: number) => {
        if (!confirm(`Kill the process on port ${port}?`)) return;
        setError('');
        try {
            await api.ports.kill(port);
            fetch();
        } catch (e: any) {
            setError(e.message);
        }
    };

    return (
        <div class="table-container">
            {error && <div style="color:var(--danger);padding:0.5rem 1rem;font-size:0.85rem">{error}</div>}
            <table>
                <thead>
                    <tr>
                        <th>Port</th>
                        <th>Protocol</th>
                        <th>PID</th>
                        <th>Process</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {loading && <tr><td colspan={5} style="text-align:center;color:var(--text3)">Loading...</td></tr>}
                    {!loading && ports.length === 0 && (
                        <tr><td colspan={5} style="text-align:center;color:var(--text3)">No listening ports found</td></tr>
                    )}
                    {ports.map(p => (
                        <tr key={`${p.port}-${p.protocol}`}>
                            <td class="mono"><strong>{p.port}</strong></td>
                            <td><span class={`state-badge ${p.protocol === 'TCP' ? 'state-running' : ''}`}>{p.protocol}</span></td>
                            <td class="mono">{p.pid}</td>
                            <td class="mono">{p.process}</td>
                            <td>
                                <button class="btn btn-sm btn-danger" onClick={() => kill(p.port)}>
                                    Kill
                                </button>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
