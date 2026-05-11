import { useEffect, useRef, useCallback } from 'preact/hooks';
import type { WsMessage } from '../types';

export function useWebSocket(onMessage: (msg: WsMessage) => void) {
    const wsRef = useRef<WebSocket | null>(null);
    const reconnectRef = useRef<ReturnType<typeof setTimeout>>();

    const connect = useCallback(() => {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${location.host}/ws`);
        wsRef.current = ws;

        ws.onopen = () => console.log('[WS] Connected');
        ws.onmessage = (e) => {
            try { onMessage(JSON.parse(e.data)); } catch {}
        };
        ws.onclose = () => {
            console.log('[WS] Disconnected, reconnecting...');
            reconnectRef.current = setTimeout(connect, 3000);
        };
    }, [onMessage]);

    useEffect(() => {
        connect();
        return () => {
            wsRef.current?.close();
            clearTimeout(reconnectRef.current);
        };
    }, [connect]);

    return wsRef;
}
