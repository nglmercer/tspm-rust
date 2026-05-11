/**
 * Process related types
 */
export type ProcessState = 'running' | 'stopped' | 'errored' | 'restarting';

export interface ProcessStatus {
  name: string;
  pid?: number;
  state: ProcessState;
  restartCount: number;
  uptime: number;
  instanceId: number;
  cpu: number;
  memory: number;
}

export interface ProcessLogEntry {
  timestamp: string;
  processName: string;
  message: string;
  type: 'stdout' | 'stderr';
}

export interface ProcessConfig {
  name: string;
  script: string;
  args?: string[];
  interpreter?: string;
  env?: Record<string, string>;
  cwd?: string;
  autorestart?: boolean;
  watch?: boolean;
  instances?: number;
  maxRestarts?: number;
  namespace?: string;
  stdout?: string;
  stderr?: string;
  install?: string;
  build?: string;
  dotEnv?: string;
  preStart?: string;
  postStart?: string;
  maxMemory?: number;
  nice?: number;
  killTimeout?: number;
  healthCheck?: HealthCheckConfig;
}

export interface HealthCheckConfig {
  enabled?: boolean;
  protocol?: 'http' | 'https' | 'tcp';
  host?: string;
  port?: number;
  path?: string;
  interval_ms?: number;
  timeout_ms?: number;
  retries?: number;
}

/**
 * System and Network types
 */
export interface PortInfo {
  port: number;
  pid: number;
  process: string;
  protocol: string;
}

export interface SystemStats {
  cpu: number;
  memory: number;
  uptime: number;
  processCount: number;
  cwd?: string;
}

/**
 * API and WebSocket types
 */
export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

export type WsMessage =
  | { type: 'process:update'; payload: { processes: ProcessStatus[] } }
  | { type: 'process:log'; payload: ProcessLogEntry }
  | { type: 'system:stats'; payload: SystemStats };

/**
 * UI State types
 */
export type AppPage = 'processes' | 'logs' | 'terminal' | 'ports';

export interface DialogOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
}

export interface DialogState extends DialogOptions {
  show: boolean;
  type: 'alert' | 'confirm';
  onConfirm?: () => void;
  onCancel?: () => void;
}

/**
 * Form definitions
 */
export interface FormField {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'string[]' | 'select';
  required: boolean;
  label: string;
  placeholder?: string;
  description?: string;
  defaultValue?: unknown;
  options?: { value: string; label: string }[];
  group: string;
  path?: boolean;
}
