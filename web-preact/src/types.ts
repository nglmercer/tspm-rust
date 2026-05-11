export interface ProcessStatus {
  name: string;
  pid?: number;
  state: string;
  restartCount: number;
  uptime: number;
  instanceId: number;
  cpu: number;
  memory: number;
}

export interface ProcessLogEntry {
  timestamp?: string;
  processName?: string;
  message?: string;
  type?: string;
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
  protocol?: string;
  host?: string;
  port?: number;
  path?: string;
  interval_ms?: number;
  timeout_ms?: number;
  retries?: number;
}

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
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

export interface WsMessage {
  type: 'process:update' | 'process:log' | 'system:stats';
  payload: any;
}

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
