import type { SystemStats } from '../types';

export function StatsBar({ stats }: { stats: SystemStats }) {
    return (
        <div class="stats-bar">
            <div class="stat"><span>Processes</span><span class="stat-value">{stats.processCount}</span></div>
            <div class="stat"><span>CPU</span><span class="stat-value">{stats.cpu.toFixed(1)}%</span></div>
            <div class="stat"><span>Memory</span><span class="stat-value">{stats.memory > 0 ? `${(stats.memory / 1024 / 1024).toFixed(0)} MB` : '—'}</span></div>
            <div class="stat"><span>Uptime</span><span class="stat-value">{Math.floor(stats.uptime / 3600)}h {Math.floor(stats.uptime / 60) % 60}m</span></div>
        </div>
    );
}
