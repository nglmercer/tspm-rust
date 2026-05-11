import type { ProcessStatus } from '../types';
import styles from '@/styles/ProcessTable.module.css';

interface Props {
    processes: ProcessStatus[];
    onStart: (name: string) => void;
    onStop: (name: string) => void;
    onRestart: (name: string) => void;
    onDelete: (name: string) => void;
}

function formatUptime(secs: number): string {
    if (secs < 60) return `${secs}s`;
    const m = Math.floor(secs / 60);
    const h = Math.floor(m / 60);
    if (h > 0) return `${h}h ${m % 60}m`;
    return `${m}m ${secs % 60}s`;
}

function formatMemory(bytes: number): string {
    if (bytes === 0) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ProcessTable({ processes, onStart, onStop, onRestart, onDelete }: Props) {
    if (processes.length === 0) {
        return (
            <div class="empty">
                <span style="font-size:2rem">📦</span>
                <p>No processes running</p>
                <p style="font-size:0.78rem">Click "+ New Process" to add one</p>
            </div>
        );
    }

    return (
        <div class="table-container">
            <table>
                <thead>
                    <tr>
                        <th>Name</th>
                        <th>Status</th>
                        <th>PID</th>
                        <th>CPU</th>
                        <th>Memory</th>
                        <th>Uptime</th>
                        <th>Restarts</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {processes.map(p => (
                        <tr key={p.name}>
                            <td><strong>{p.name}</strong></td>
                            <td><span class={`state-badge state-${p.state}`}>{p.state}</span></td>
                            <td class="mono">{p.pid ?? '-'}</td>
                            <td class="mono">{p.cpu.toFixed(1)}%</td>
                            <td class="mono">{formatMemory(p.memory)}</td>
                            <td class="mono">{formatUptime(p.uptime)}</td>
                            <td class="mono">{p.restartCount}</td>
                            <td>
                                <div class={styles.actions}>
                                    {p.state === 'stopped' && <button class="btn btn-sm btn-primary" onClick={() => onStart(p.name)}>Start</button>}
                                    {p.state === 'running' && <button class="btn btn-sm btn-ghost" onClick={() => onStop(p.name)}>Stop</button>}
                                    {p.state === 'running' && <button class="btn btn-sm btn-ghost" onClick={() => onRestart(p.name)}>Restart</button>}
                                    <button class="btn btn-sm btn-danger" onClick={() => onDelete(p.name)}>×</button>
                                </div>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
