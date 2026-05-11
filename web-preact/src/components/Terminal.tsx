import { useState, useRef, useEffect } from 'preact/hooks';
import { api } from '../api/client';
import styles from './Terminal.module.css';

export function Terminal() {
    const [history, setHistory] = useState<string[]>(['TSPM Terminal — type commands below']);
    const [input, setInput] = useState('');
    const [cwd, setCwd] = useState('/');
    const outputRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        api.stats().then(r => {
            if (r.success && (r.data as any)?.cwd) setCwd((r.data as any).cwd);
        });
    }, []);

    useEffect(() => {
        outputRef.current?.scrollTo(0, outputRef.current.scrollHeight);
    }, [history]);

    const run = async (cmd: string) => {
        if (!cmd.trim()) return;
        setHistory(h => [...h, `${cwd}$ ${cmd}`]);
        setInput('');
        try {
            const res = await api.execute(cmd, cwd);
            if (res.output) setHistory(h => [...h, res.output.trimEnd()]);
            if (res.error) setHistory(h => [...h, ...res.error.trimEnd().split('\n').map(l => `[stderr] ${l}`)]);
            if (cmd.startsWith('cd ')) {
                api.stats().then(r => {
                    if (r.success && (r.data as any)?.cwd) setCwd((r.data as any).cwd);
                });
            }
        } catch (e: any) {
            setHistory(h => [...h, `Error: ${e.message}`]);
        }
    };

    const handleKeyDown = async (e: KeyboardEvent) => {
        if (e.key === 'Enter') { e.preventDefault(); await run(input); }
        else if (e.key === 'Tab') {
            e.preventDefault();
            const prefix = input.split(' ').pop() || '';
            const suggestions = await api.autocomplete(prefix, cwd);
            if (suggestions.length === 1) {
                const parts = input.split(' ');
                parts[parts.length - 1] = suggestions[0];
                setInput(parts.join(' '));
            } else if (suggestions.length > 0) {
                setHistory(h => [...h, suggestions.join('  ')]);
            }
        }
    };

    return (
        <div class={styles.container}>
            <div class={styles.output} ref={outputRef}>
                {history.map((line, i) => (
                    <div key={i} style={{ color: line.startsWith('[stderr]') ? 'var(--danger)' : line.startsWith(cwd) ? 'var(--text3)' : 'inherit' }}>
                        {line}
                    </div>
                ))}
            </div>
            <div class={styles.inputRow}>
                <span class={styles.prompt}>{cwd}$</span>
                <input
                    class={styles.input}
                    value={input}
                    onInput={e => setInput((e.target as HTMLInputElement).value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Type a command..."
                    spellcheck={false}
                    autocomplete="off"
                />
            </div>
        </div>
    );
}
