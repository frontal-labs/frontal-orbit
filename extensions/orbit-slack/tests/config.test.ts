import { describe, expect, it } from 'vitest';
import config, { validateConfig } from '../src/config';

describe('config compatibility surface', () => {
  it('exports the validated runtime config object', () => {
    expect(config.slack.botToken).toEqual(expect.any(String));
    expect(config.slack.botToken.length).toBeGreaterThan(0);
    expect(config.orbit.apiUrl).toMatch(/^https?:\/\//);
    expect(config.app.nodeEnv).toEqual(expect.any(String));
  });

  it('keeps validateConfig as a no-op compatibility helper', () => {
    expect(validateConfig()).toBeUndefined();
  });
});
