import { useRef, useEffect } from 'preact/hooks';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { api } from '../api/client';
import styles from '@/styles/Terminal.module.css';

export function Terminal() {
    const terminalRef = useRef<HTMLDivElement>(null);
    const xterm = useRef<XTerm | null>(null);
    const fitAddon = useRef<FitAddon | null>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const cwdRef = useRef('/');

    useEffect(() => {
        if (!terminalRef.current) return;

        // Initialize xterm.js
        const term = new XTerm({
            cursorBlink: true,
            theme: {
                background: '#0a0a0a',
                foreground: '#ffffff',
                cursor: '#ffffff',
                selectionBackground: 'rgba(255, 255, 255, 0.3)',
                black: '#000000',
                red: '#ff5555',
                green: '#50fa7b',
                yellow: '#f1fa8c',
                blue: '#bd93f9',
                magenta: '#ff79c6',
                cyan: '#8be9fd',
                white: '#bfbfbf',
                brightBlack: '#4d4d4d',
                brightRed: '#ff6e67',
                brightGreen: '#5af78e',
                brightYellow: '#f4f99d',
                brightBlue: '#caa9fa',
                brightMagenta: '#ff92d0',
                brightCyan: '#9aedfe',
                brightWhite: '#e6e6e6',
            },
            fontSize: 14,
            fontFamily: 'JetBrains Mono, Menlo, Monaco, Consolas, "Courier New", monospace',
            convertEol: true,
        });

        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(terminalRef.current);
        fit.fit();

        term.writeln('\x1b[1;34mTSPM Terminal\x1b[0m — type commands below');
        
        xterm.current = term;
        fitAddon.current = fit;

        api.stats().then(r => {
            if (r.success && (r.data as any)?.cwd) {
                cwdRef.current = (r.data as any).cwd;
                term.write(`\x1b[1;32m${cwdRef.current}\x1b[0m$ `);
            }
        });

        const handleResize = () => fit.fit();
        window.addEventListener('resize', handleResize);

        return () => {
            window.removeEventListener('resize', handleResize);
            term.dispose();
        };
    }, []);

    const run = async (cmd: string) => {
        if (!cmd.trim() || !xterm.current) return;
        
        const term = xterm.current;
        term.writeln(''); // Move to next line after command

        try {
            const eventSource = await api.executeStream(cmd, cwdRef.current, (data) => {
                if (data.type === 'output') {
                    const payload = JSON.parse(data.data);
                    term.write(payload.data);
                } else if (data.type === 'cwd') {
                    cwdRef.current = data.data;
                } else if (data.type === 'complete') {
                    term.write(`\r\n\x1b[1;32m${cwdRef.current}\x1b[0m$ `);
                }
            });
        } catch (e: any) {
            term.writeln(`\r\n\x1b[31mError: ${e.message}\x1b[0m`);
            term.write(`\x1b[1;32m${cwdRef.current}\x1b[0m$ `);
        }
    };

    const handleKeyDown = async (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
            const input = (e.target as HTMLInputElement);
            const cmd = input.value;
            input.value = '';
            await run(cmd);
        } else if (e.key === 'Tab') {
            e.preventDefault();
            const input = (e.target as HTMLInputElement);
            const prefix = input.value.split(' ').pop() || '';
            const suggestions = await api.autocomplete(prefix, cwdRef.current);
            if (suggestions.length === 1) {
                const parts = input.value.split(' ');
                parts[parts.length - 1] = suggestions[0];
                input.value = parts.join(' ');
            } else if (suggestions.length > 0) {
                xterm.current?.writeln(`\r\n${suggestions.join('  ')}`);
                xterm.current?.write(`\x1b[1;32m${cwdRef.current}\x1b[0m$ `);
            }
        }
    };

    return (
        <div class={styles.container}>
            <div class={styles.xtermContainer} ref={terminalRef} />
            <div class={styles.inputRow}>
                <span class={styles.prompt}>{cwdRef.current}$</span>
                <input
                    ref={inputRef}
                    class={styles.input}
                    onKeyDown={handleKeyDown}
                    placeholder="Type a command..."
                    spellcheck={false}
                    autocomplete="off"
                    autoFocus
                />
            </div>
        </div>
    );
}
