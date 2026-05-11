import { useState, useCallback } from 'preact/hooks';
import { api } from '../api/client';
import type { ProcessConfig } from '../types';
import { IconFolder, IconFile } from './Icons';
import styles from '@/styles/ProcessForm.module.css';

interface Props {
    onClose: () => void;
}

const fields: { name: string; label: string; placeholder?: string; type?: string }[] = [
    { name: 'name', label: 'Name', placeholder: 'my-server' },
    { name: 'script', label: 'Script / Command', placeholder: 'bun run server.ts' },
    { name: 'args', label: 'Arguments', placeholder: '--port 3000' },
    { name: 'cwd', label: 'Working Directory', placeholder: '/path/to/project' },
    { name: 'interpreter', label: 'Interpreter', placeholder: 'bun / node / python' },
    { name: 'instances', label: 'Instances', type: 'number', placeholder: '1' },
    { name: 'namespace', label: 'Namespace', placeholder: 'production' },
    { name: 'stdout', label: 'Stdout Log', placeholder: 'logs/app.log' },
    { name: 'stderr', label: 'Stderr Log', placeholder: 'logs/app-err.log' },
    { name: 'install', label: 'Install Script', placeholder: 'npm install' },
    { name: 'build', label: 'Build Script', placeholder: 'npm run build' },
    { name: 'preStart', label: 'Pre-start Script', placeholder: 'echo starting...' },
    { name: 'postStart', label: 'Post-start Script', placeholder: 'echo started!' },
    { name: 'maxRestarts', label: 'Max Restarts', type: 'number', placeholder: '10' },
    { name: 'maxMemory', label: 'Max Memory (bytes)', type: 'number', placeholder: '0 = disabled' },
];

const pathFields = new Set(['script', 'cwd', 'stdout', 'stderr']);

export function ProcessForm({ onClose }: Props) {
    const [form, setForm] = useState<Record<string, string>>({});
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [activeField, setActiveField] = useState('');
    const [error, setError] = useState('');

    const handleInput = useCallback(async (field: string, value: string) => {
        setForm(f => ({ ...f, [field]: value }));
        setError('');

        if (pathFields.has(field) && value) {
            const suggestions = await api.autocomplete(value, form.cwd || '.');
            setSuggestions(suggestions);
            setActiveField(field);
        } else {
            setSuggestions([]);
            setActiveField('');
        }
    }, [form.cwd]);

    const applySuggestion = (s: string) => {
        const current = form[activeField] || '';
        const lastSep = current.lastIndexOf('/');
        const updated = lastSep >= 0 ? current.substring(0, lastSep + 1) + s : s;
        setForm(f => ({ ...f, [activeField]: updated }));
        setSuggestions([]);
        setActiveField('');
    };

    const handleSubmit = async (e: Event) => {
        e.preventDefault();
        setError('');
        if (!form.name?.trim()) { setError('Name is required'); return; }
        if (!form.script?.trim()) { setError('Script is required'); return; }

        const config: ProcessConfig = {
            name: form.name.trim(),
            script: form.script.trim(),
            args: form.args?.split(' ').filter(Boolean),
            cwd: form.cwd?.trim() || undefined,
            interpreter: form.interpreter?.trim() || undefined,
            instances: form.instances ? parseInt(form.instances) : undefined,
            namespace: form.namespace?.trim() || undefined,
            stdout: form.stdout?.trim() || undefined,
            stderr: form.stderr?.trim() || undefined,
            install: form.install?.trim() || undefined,
            build: form.build?.trim() || undefined,
            preStart: form.preStart?.trim() || undefined,
            postStart: form.postStart?.trim() || undefined,
            maxRestarts: form.maxRestarts ? parseInt(form.maxRestarts) : undefined,
            maxMemory: form.maxMemory ? parseInt(form.maxMemory) : undefined,
        };

        try {
            await api.processes.create(config);
            onClose();
        } catch (e: any) {
            setError(e.message || 'Failed to create process');
        }
    };

    return (
        <div class="modal-overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
            <form class={styles.modal} onSubmit={handleSubmit}>
                <h2>New Process</h2>

                {error && <div style="color:var(--danger);margin-bottom:0.8rem;font-size:0.85rem">{error}</div>}

                <div style="display:grid;grid-template-columns:1fr 1fr;gap:0 1rem">
                    {fields.map(f => {
                        const isPath = pathFields.has(f.name);
                        return (
                            <div class={styles.formGroup} key={f.name} style={f.name === 'script' || f.name === 'name' ? 'grid-column:1/-1' : ''}>
                                <label>{f.label}</label>
                                <div class={styles.autocompleteWrap}>
                                    <input
                                        type={f.type === 'number' ? 'number' : 'text'}
                                        placeholder={f.placeholder}
                                        value={form[f.name] || ''}
                                        onInput={e => handleInput(f.name, (e.target as HTMLInputElement).value)}
                                        autocomplete="off"
                                    />
                                    {isPath && activeField === f.name && suggestions.length > 0 && (
                                        <ul class={styles.autocompleteDrop}>
                                            {suggestions.map((s, i) => (
                                                <li key={i} onMouseDown={e => { e.preventDefault(); applySuggestion(s); }}>
                                                    {s.endsWith('/') ? <IconFolder size={14} /> : <IconFile size={14} />} {s}
                                                </li>
                                            ))}
                                        </ul>
                                    )}
                                </div>
                            </div>
                        );
                    })}
                </div>

                <div class={styles.formActions}>
                    <button type="button" class="btn btn-ghost" onClick={onClose}>Cancel</button>
                    <button type="submit" class="btn btn-primary">Create Process</button>
                </div>
            </form>
        </div>
    );
}
