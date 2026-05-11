import { useState, useEffect, useCallback } from 'preact/hooks';
import { api } from '../api/client';
import type { SystemStats } from '../types';

export function useSystemStats() {
    const [stats, setStats] = useState<SystemStats>({
        cpu: 0,
        memory: 0,
        uptime: 0,
        processCount: 0
    });
    const [loading, setLoading] = useState(true);

    const fetchStats = useCallback(async () => {
        try {
            const res = await api.stats();
            if (res.success && res.data) {
                setStats(res.data);
            }
        } catch (e) {
            console.error('Failed to fetch system stats', e);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        fetchStats();
        // Fallback polling if WS is not used, or just for initial load
        const timer = setInterval(fetchStats, 10000);
        return () => clearInterval(timer);
    }, [fetchStats]);

    const updateFromWs = useCallback((newStats: SystemStats) => {
        setStats(newStats);
    }, []);

    return { stats, loading, refresh: fetchStats, updateFromWs };
}
