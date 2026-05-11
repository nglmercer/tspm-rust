import type { 
    ProcessStatus, 
    ProcessLogEntry, 
    ProcessConfig, 
    SystemStats, 
    PortInfo, 
    ApiResponse 
} from '../types';

const BASE = '/api/v1';

async function request<T>(path: string, options?: RequestInit): Promise<ApiResponse<T>> {
    const res = await fetch(`${BASE}${path}`, {
        headers: { 'Content-Type': 'application/json' },
        ...options,
    });
    const json = await res.json();
    if (!res.ok) throw new Error(json.error || `HTTP ${res.status}`);
    return json as ApiResponse<T>;
}

export const api = {
    // ─── Processes ──────────────────────────────────────
    processes: {
        list(): Promise<ApiResponse<ProcessStatus[]>> {
            return request('/processes');
        },
        get(name: string): Promise<ApiResponse<ProcessStatus>> {
            return request(`/processes/${encodeURIComponent(name)}`);
        },
        create(config: ProcessConfig): Promise<ApiResponse<null>> {
            return request('/processes', { method: 'POST', body: JSON.stringify(config) });
        },
        delete(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}`, { method: 'DELETE' });
        },
        start(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/start`, { method: 'POST' });
        },
        stop(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/stop`, { method: 'POST' });
        },
        restart(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/restart`, { method: 'POST' });
        },
        logs(name: string, limit = 200): Promise<ApiResponse<{ logs: ProcessLogEntry[] }>> {
            return request(`/processes/${encodeURIComponent(name)}/logs?limit=${limit}`);
        },
        install(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/install`, { method: 'POST' });
        },
        build(name: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/build`, { method: 'POST' });
        },
        sendInput(name: string, input: string): Promise<ApiResponse<null>> {
            return request(`/processes/${encodeURIComponent(name)}/input`, {
                method: 'POST',
                body: JSON.stringify({ input }),
            });
        },
    },

    // ─── System ─────────────────────────────────────────
    status(): Promise<ApiResponse<{ processes: ProcessStatus[]; stats: SystemStats }>> {
        return request('/status');
    },
    stats(): Promise<ApiResponse<SystemStats>> {
        return request('/stats');
    },
    health(): Promise<ApiResponse<{ status: string }>> {
        return request('/health');
    },
    logs(limit = 200): Promise<ApiResponse<{ logs: ProcessLogEntry[] }>> {
        return request(`/logs?limit=${limit}`);
    },
    async execute(command: string, cwd?: string): Promise<{ output: string; error: string; exitCode: number }> {
        const res = await fetch(`${BASE}/execute`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ command, cwd: cwd || '.', stream: false }),
        });
        return res.json();
    },

    // ─── Autocomplete ──────────────────────────────────
    async autocomplete(prefix: string, cwd: string): Promise<string[]> {
        const res = await fetch(`${BASE}/autocomplete`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ prefix, cwd }),
        });
        const data = await res.json();
        return data.success ? (data.suggestions ?? []) : [];
    },

    // ─── Dump / Persistence ────────────────────────────
    dump: {
        get(): Promise<ApiResponse<{ processes: ProcessConfig[] }>> {
            return request('/dump');
        },
        save(processes: ProcessConfig[]): Promise<ApiResponse<null>> {
            return request('/dump', { method: 'PUT', body: JSON.stringify({ processes }) });
        },
        delete(name: string): Promise<ApiResponse<null>> {
            return request(`/dump/${encodeURIComponent(name)}`, { method: 'DELETE' });
        },
    },
    // ─── Ports ─────────────────────────────────────────
    ports: {
        list(): Promise<ApiResponse<PortInfo[]>> {
            return request('/ports');
        },
        kill(port: number): Promise<ApiResponse<{ pid: number; process: string }>> {
            return request(`/ports/${port}`, { method: 'POST' });
        },
    },
};
