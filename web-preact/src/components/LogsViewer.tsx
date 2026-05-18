import { useState, useRef, useEffect, useMemo } from 'preact/hooks';
import type { ProcessLogEntry, ProcessStatus } from '../types';
import { AnsiText } from './AnsiText';
import styles from '@/styles/LogsViewer.module.css';

interface Props {
    entries: ProcessLogEntry[];
    processes: ProcessStatus[];
    onClear: () => void;
}

interface LogGroup {
    entry: ProcessLogEntry;
    count: number;
}

function groupLogs(entries: ProcessLogEntry[]): LogGroup[] {
    if (entries.length === 0) return [];
    const groups: LogGroup[] = [];
    let current: LogGroup = { entry: entries[0], count: 1 };
    for (let i = 1; i < entries.length; i++) {
        const e = entries[i];
        if (e.message === current.entry.message && e.type === current.entry.type && e.processName === current.entry.processName) {
            current.count++;
        } else {
            groups.push(current);
            current = { entry: e, count: 1 };
        }
    }
    groups.push(current);
    return groups;
}

export function LogsViewer({ entries, processes, onClear }: Props) {
    const [filter, setFilter] = useState('all');
    const [paused, setPaused] = useState(false);
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!paused) bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [entries.length, paused]);

    const filtered = useMemo(() => {
        const list = filter === 'all'
            ? entries
            : entries.filter(e => e.processName === filter || e.processName?.split('/').pop() === filter);
        return groupLogs(list);
    }, [entries, filter]);

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
                    <span class="state-badge">{entries.length} lines</span>
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
                {filtered.map((g, i) => (
                    <div class={`${styles.line} ${g.count > 1 ? styles.grouped : ''}`} key={i}>
                        <span class={styles.time}>{g.entry.timestamp || '--:--:--'}</span>
                        <span class={styles.proc} title={g.entry.processName}>{g.entry.processName?.split('/').pop() || g.entry.processName}</span>
                        <span class={`${styles.msg} ${g.entry.type === 'stderr' ? styles.stderr : ''}`}>
                            <AnsiText text={g.entry.message} />
                        </span>
                        {g.count > 1 && <span class={styles.badge}>×{g.count}</span>}
                    </div>
                ))}
                <div ref={bottomRef} />
            </div>
        </div>
    );
}
