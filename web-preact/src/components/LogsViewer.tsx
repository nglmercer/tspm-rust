import { useState, useRef, useEffect } from 'preact/hooks';
import type { ProcessLogEntry, ProcessStatus } from '../types';
import { AnsiText } from './AnsiText';
import styles from '@/styles/LogsViewer.module.css';

interface Props {
    entries: ProcessLogEntry[];
    processes: ProcessStatus[];
    onClear: () => void;
}

export function LogsViewer({ entries, processes, onClear }: Props) {
    const [filter, setFilter] = useState('all');
    const [paused, setPaused] = useState(false);
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!paused) bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [entries, paused]);

    const filtered = filter === 'all'
        ? entries
        : entries.filter(e => e.processName === filter || e.processName?.split('/').pop() === filter);

    return (
        <div class={styles.container}>
            <div class={styles.header}>
                <div style="display:flex;gap:0.5rem;align-items:center">
                    <select
                        value={filter}
                        onChange={e => setFilter((e.target as HTMLSelectElement).value)}
                        style="padding:4px 8px;background:var(--surface2);border:1px solid var(--border);border-radius:6px;color:var(--text);font:inherit;font-size:0.82rem"
                    >
                        <option value="all">All processes</option>
                        {processes.map(p => <option value={p.name} key={p.name}>{p.name}</option>)}
                    </select>
                    <span class="state-badge">{filtered.length} lines</span>
                </div>
                <div style="display:flex;gap:0.5rem">
                    <button class="btn btn-sm btn-ghost" onClick={() => setPaused(!paused)}>
                        {paused ? '▶ Resume' : '⏸ Pause'}
                    </button>
                    <button class="btn btn-sm btn-ghost" onClick={onClear}>Clear</button>
                </div>
            </div>
            <div class={styles.output}>
                {filtered.length === 0 && <div class="empty"><p>No logs yet</p></div>}
                {filtered.map((e, i) => (
                    <div class={styles.line} key={i}>
                        <span class={styles.time}>{e.timestamp || '--:--:--'}</span>
                        <span class={styles.proc} title={e.processName}>{e.processName?.split('/').pop() || e.processName}</span>
                        <span class={`${styles.msg} ${e.type === 'stderr' ? styles.stderr : ''}`}>
                            <AnsiText text={e.message} />
                        </span>
                    </div>
                ))}
                <div ref={bottomRef} />
            </div>
        </div>
    );
}
