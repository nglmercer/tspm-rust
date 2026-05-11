import type { SystemStats } from '../types';
import styles from '@/styles/StatsBar.module.css';

export function StatsBar({ stats }: { stats: SystemStats }) {
    return (
        <div class={styles.statsBar}>
            <div class={styles.stat}><span>Processes</span><span class={styles.statValue}>{stats.processCount}</span></div>
            <div class={styles.stat}><span>CPU</span><span class={styles.statValue}>{stats.cpu.toFixed(1)}%</span></div>
            <div class={styles.stat}><span>Memory</span><span class={styles.statValue}>{stats.memory > 0 ? `${(stats.memory / 1024 / 1024).toFixed(0)} MB` : '—'}</span></div>
            <div class={styles.stat}><span>Uptime</span><span class={styles.statValue}>{Math.floor(stats.uptime / 3600)}h {Math.floor(stats.uptime / 60) % 60}m</span></div>
        </div>
    );
}
