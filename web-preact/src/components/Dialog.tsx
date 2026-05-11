import { useState, useEffect } from 'preact/hooks';
import styles from './Dialog.module.css';
import type { DialogState } from '../types';

let dialogSetter: (state: DialogState) => void = () => {};

export const dialog = {
    alert: (title: string, message: string, confirmText = 'OK') => {
        return new Promise<void>((resolve) => {
            dialogSetter({
                show: true,
                type: 'alert',
                title,
                message,
                confirmText,
                onConfirm: () => resolve(),
            });
        });
    },
    confirm: (title: string, message: string, confirmText = 'Confirm', cancelText = 'Cancel') => {
        return new Promise<boolean>((resolve) => {
            dialogSetter({
                show: true,
                type: 'confirm',
                title,
                message,
                confirmText,
                cancelText,
                onConfirm: () => resolve(true),
                onCancel: () => resolve(false),
            });
        });
    }
};

export function Dialog() {
    const [state, setState] = useState<DialogState>({
        show: false,
        type: 'alert',
        title: '',
        message: '',
    });

    useEffect(() => {
        dialogSetter = setState;
    }, []);

    if (!state.show) return null;

    const handleAction = (confirmed: boolean) => {
        const { onConfirm, onCancel } = state;
        setState(s => ({ ...s, show: false }));
        if (confirmed) {
            onConfirm?.();
        } else {
            onCancel?.();
        }
    };

    return (
        <div class={`modal-overlay ${styles.overlay}`} onClick={() => state.type === 'alert' && handleAction(true)}>
            <div class={styles.card} onClick={e => e.stopPropagation()}>
                <div class={styles.header}>
                    <div class={`${styles.icon} ${state.type === 'alert' ? styles.iconAlert : styles.iconConfirm}`}>
                        {state.type === 'alert' ? 'ℹ️' : '❓'}
                    </div>
                    <h3 class={styles.title}>{state.title}</h3>
                </div>
                <div class={styles.body}>
                    <p>{state.message}</p>
                </div>
                <div class={styles.footer}>
                    {state.type === 'confirm' && (
                        <button class="btn btn-ghost" onClick={() => handleAction(false)}>
                            {state.cancelText || 'Cancel'}
                        </button>
                    )}
                    <button 
                        class={`btn ${state.type === 'confirm' ? 'btn-danger' : 'btn-primary'}`} 
                        onClick={() => handleAction(true)}
                    >
                        {state.confirmText || (state.type === 'confirm' ? 'Confirm' : 'OK')}
                    </button>
                </div>
            </div>
        </div>
    );
}
