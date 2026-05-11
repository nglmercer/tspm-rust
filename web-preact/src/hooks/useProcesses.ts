import { useState, useEffect, useCallback } from 'preact/hooks';
import { api } from '../api/client';
import type { ProcessStatus } from '../types';

export function useProcesses() {
    const [processes, setProcesses] = useState<ProcessStatus[]>([]);
    const [loading, setLoading] = useState(true);

    const fetch = useCallback(async () => {
        try {
            const res = await api.status();
            if (res.success) {
                setProcesses(res.data?.processes ?? []);
            }
        } catch (e) {
            console.error('Failed to fetch processes', e);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => { fetch(); }, [fetch]);

    const updateFromWs = useCallback((procs: ProcessStatus[]) => {
        setProcesses(procs);
    }, []);

    const startProcess = useCallback(async (name: string) => {
        await api.processes.start(name);
        fetch();
    }, [fetch]);

    const stopProcess = useCallback(async (name: string) => {
        await api.processes.stop(name);
        fetch();
    }, [fetch]);

    const restartProcess = useCallback(async (name: string) => {
        await api.processes.restart(name);
        fetch();
    }, [fetch]);

    const deleteProcess = useCallback(async (name: string) => {
        if (!confirm(`Delete process "${name}"?`)) return;
        await api.processes.delete(name);
        fetch();
    }, [fetch]);

    return { processes, loading, fetch, updateFromWs, startProcess, stopProcess, restartProcess, deleteProcess };
}
