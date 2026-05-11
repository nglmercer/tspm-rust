import { JSX, type AllCSSProperties } from 'preact';

interface Props {
    text: string;
    style?: AllCSSProperties;
    highlight?: boolean;
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

const HIGHLIGHT_RULES = [
    { regex: /https?:\/\/[^\s)]+/g, color: 'var(--primary)', decoration: 'underline' }, // URLs
    { regex: /\b(ERROR|FAIL|FAILED|FATAL)\b/gi, color: 'var(--danger)', weight: 'bold' }, // Errors
    { regex: /\b(SUCCESS|OK|DONE|PASSED|COMPLETED)\b/gi, color: 'var(--success)', weight: 'bold' }, // Success
    { regex: /\b(WARN|WARNING|ATTENTION)\b/gi, color: 'var(--warning)', weight: 'bold' }, // Warnings
    { regex: /\b\d+(\.\d+)?\b/g, color: 'var(--success)' }, // Numbers
    { regex: /"[^"]*"|'[^']*'/g, color: 'var(--warning)' }, // Strings
    { regex: /\/[a-zA-Z0-9._/-]+/g, color: 'var(--cyan)' }, // Absolute paths
    { regex: /#[a-zA-Z0-9_-]+/g, color: 'var(--magenta)' }, // Tags/Hashes
];

function renderHighlighted(text: string): (JSX.Element | string)[] {
    let parts: (JSX.Element | string)[] = [text];

    for (const rule of HIGHLIGHT_RULES) {
        const nextParts: (JSX.Element | string)[] = [];
        for (const part of parts) {
            if (typeof part !== 'string') {
                nextParts.push(part);
                continue;
            }

            let lastIndex = 0;
            let match;
            rule.regex.lastIndex = 0;

            while ((match = rule.regex.exec(part)) !== null) {
                if (match.index > lastIndex) {
                    nextParts.push(part.substring(lastIndex, match.index));
                }
                nextParts.push(
                    <span style={{
                        color: rule.color,
                        fontWeight: rule.weight || 'inherit',
                        textDecoration: rule.decoration || 'none'
                    }}>
                        {match[0]}
                    </span>
                );
                lastIndex = rule.regex.lastIndex;
            }

            if (lastIndex < part.length) {
                nextParts.push(part.substring(lastIndex));
            }
        }
        parts = nextParts;
    }

    return parts;
}

export function AnsiText({ text, style, highlight = true }: Props) {
    const hasAnsi = text.includes('\x1b') || text.includes('\u001b');

    if (!hasAnsi) {
        return (
            <span style={style}>
                {highlight ? renderHighlighted(text) : text}
            </span>
        );
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

                const spanStyle: AllCSSProperties = {
                    color: currentColor || 'inherit',
                    fontWeight: isBold ? 'bold' : 'inherit',
                };

                // Even for ANSI text, we can apply sub-highlighting to the content part if it's plain
                return (
                    <span key={i} style={spanStyle}>
                        {highlight && !currentColor ? renderHighlighted(part) : part}
                    </span>
                );
            })}
        </span>
    );
}
