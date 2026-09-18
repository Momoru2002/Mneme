import { beforeEach, describe, expect, it } from 'vitest';
import { useUiStore } from './ui.ts';

beforeEach(() => {
  useUiStore.setState({
    sidebarView: 'files',
    settingsOpenNonce: 0,
    newFileNonce: 0,
    sidebarCollapsed: false,
  });
});

describe('ui store — sidebarView', () => {
  it('defaults to the files view', () => {
    expect(useUiStore.getState().sidebarView).toBe('files');
  });

  it('setSidebarView switches the view the sidebar shows', () => {
    useUiStore.getState().setSidebarView('search');
    expect(useUiStore.getState().sidebarView).toBe('search');
    useUiStore.getState().setSidebarView('files');
    expect(useUiStore.getState().sidebarView).toBe('files');
  });
});

describe('ui store — requestSettingsOpen', () => {
  it('increments the settings nonce each call', () => {
    const start = useUiStore.getState().settingsOpenNonce;
    useUiStore.getState().requestSettingsOpen();
    useUiStore.getState().requestSettingsOpen();
    expect(useUiStore.getState().settingsOpenNonce).toBe(start + 2);
  });
});

describe('ui store — requestNewFile', () => {
  it('increments the new-file nonce each call', () => {
    const start = useUiStore.getState().newFileNonce;
    useUiStore.getState().requestNewFile();
    useUiStore.getState().requestNewFile();
    expect(useUiStore.getState().newFileNonce).toBe(start + 2);
  });
});

describe('ui store — sidebar visibility', () => {
  it('toggleSidebar flips collapsed state', () => {
    expect(useUiStore.getState().sidebarCollapsed).toBe(false);
    useUiStore.getState().toggleSidebar();
    expect(useUiStore.getState().sidebarCollapsed).toBe(true);
    useUiStore.getState().toggleSidebar();
    expect(useUiStore.getState().sidebarCollapsed).toBe(false);
  });

  it('setSidebarCollapsed sets the state directly', () => {
    useUiStore.getState().setSidebarCollapsed(true);
    expect(useUiStore.getState().sidebarCollapsed).toBe(true);
    useUiStore.getState().setSidebarCollapsed(false);
    expect(useUiStore.getState().sidebarCollapsed).toBe(false);
  });
});
