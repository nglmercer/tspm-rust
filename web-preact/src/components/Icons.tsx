import { SVGAttributes } from 'preact';

interface IconProps extends SVGAttributes<SVGSVGElement> {
    size?: number | string;
}

const DefaultProps: SVGAttributes<SVGSVGElement> = {
    xmlns: "http://www.w3.org/2000/svg",
    width: "24",
    height: "24",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
    strokeLinecap: "round",
    strokeLinejoin: "round",
};

export const IconActivity = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
    </svg>
);

export const IconTerminal = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
);

export const IconFileText = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10 9 9 9 8 9" />
    </svg>
);

export const IconPlug = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <path d="M12 2v5" />
        <path d="M9 2v5" />
        <path d="M6 7h12a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2z" />
        <path d="M12 15v3a2 2 0 0 0 2 2h3a2 2 0 0 0 2-2v-3" />
    </svg>
);

export const IconZap = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
);

export const IconPlus = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
);

export const IconRefresh = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polyline points="23 4 23 10 17 10" />
        <polyline points="1 20 1 14 7 14" />
        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
    </svg>
);

export const IconTrash = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polyline points="3 6 5 6 21 6" />
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        <line x1="10" y1="11" x2="10" y2="17" />
        <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
);

export const IconPlay = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <polygon points="5 3 19 12 5 21 5 3" />
    </svg>
);

export const IconSquare = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
    </svg>
);

export const IconFolder = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z" />
    </svg>
);

export const IconFile = ({ size = 20, ...props }: IconProps) => (
    <svg {...DefaultProps} width={size} height={size} {...props}>
        <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
        <polyline points="14 2 14 8 20 8" />
    </svg>
);
