import { afterEach, expect, test } from 'vitest';
import { setLocale, t } from '../src/lib/i18n';
import { formatItemName, formatSlotName } from '../src/modules/dashboard/shipFormatters';

afterEach(() => setLocale('es'));

test('module labels follow the current locale and the engineering catalogue', () => {
  for (const lang of ['es', 'en', 'fr', 'de', 'pt']) {
    setLocale(lang);
    expect(formatItemName('int_hyperdrive_size5_class5')).toEqual({ label: t('modtype-fsd'), sizeClass: '5A' });
    expect(formatItemName('int_fuelscoop_size4_class3')).toEqual({ label: t('modtype-fuelscoop'), sizeClass: '4C' });
    expect(formatItemName('hpt_fragmentcannon_fixed_size3_class2').label).toBe(t('modtype-fragmentcannon'));
    expect(formatSlotName('FrameShiftDrive')).toBe(t('modtype-fsd'));
  }
});

test('slot numbers, capacities, mounts and unknown symbols remain intact', () => {
  setLocale('en');
  expect(formatSlotName('LargeHardpoint2')).toBe('Large hardpoint 2');
  expect(formatSlotName('Slot03_Size6')).toBe('Slot 03 (size 6)');
  expect(formatItemName('int_cargorack_size4_class1').label).toBe('Cargo Rack (16 t)');
  expect(formatItemName('hpt_multicannon_gimbal_size2_class2').label).toBe(`Multi-cannon (${t('outfitting-mount-gimbal')})`);
  expect(formatSlotName('FutureSlot')).toBe('FutureSlot');
  expect(formatItemName('int_future_device_size2_class5').sizeClass).toBe('2A');
});
