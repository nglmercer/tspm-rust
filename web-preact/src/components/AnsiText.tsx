import { JSX } from 'preact';

interface Props {
    text: string;
    style?: JSX.CSSProperties;
}

const ANSI_COLORS: Record<string, string> = {
    '30': 'var(--black, #000)',
    '31': 'var(--danger, #f44336)',
    '32': 'var(--success, #4caf50)',
    '33': 'var(--warning, #ffeb3b)',
    '34': 'var(--primary, #2196f3)',
    '35': 'var(--magenta, #9c27b0)',
    '36': 'var(--cyan, #00bcd4)',
    '37': 'var(--white, #fff)',
    '90': 'var(--gray, #9e9e9e)',
};

export function AnsiText({ text, style }: Props) {
    if (!text.includes('\x1b') && !text.includes('\u001b')) {
        return <span style={style}>{text}</span>;
    }

    const parts = text.split(/(\x1b\[[0-9;]*m)/);
    let currentColor = '';
    let isBold = false;

    return (
        <span style={style}>
            {parts.map((part, i) => {
                const match = part.match(/\x1b\[([0-9;]*)m/);
                if (match) {
                    const codes = match[1].split(';');
                    for (const code of codes) {
                        if (code === '0') {
                            currentColor = '';
                            isBold = false;
                        } else if (code === '1') {
                            isBold = true;
                        } else if (ANSI_COLORS[code]) {
                            currentColor = ANSI_COLORS[code];
                        }
                    }
                    return null;
                }

                if (!part) return null;

                const spanStyle: JSX.CSSProperties = {
                    color: currentColor || 'inherit',
                    fontWeight: isBold ? 'bold' : 'inherit',
                };

                return <span key={i} style={spanStyle}>{part}</span>;
            })}
        </span>
    );
}
