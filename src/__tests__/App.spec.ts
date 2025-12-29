import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import App from '../App.vue';

describe('App', () => {
  it('renders properly', () => {
    const wrapper = mount(App);
    expect(wrapper.text()).toContain('CUK');
  });

  it('shows phase 1 message', () => {
    const wrapper = mount(App);
    expect(wrapper.text()).toContain('Phase 1');
  });
});
