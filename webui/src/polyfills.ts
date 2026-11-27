// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

// eslint-disable-next-line @typescript-eslint/no-explicit-any
if (!(Object as any).hasOwn) {
  Object.defineProperty(Object, 'hasOwn', {
    value(object: unknown, property: PropertyKey) {
      if (object == null) {
        throw new TypeError('Cannot convert undefined or null to object')
      }
      return Object.prototype.hasOwnProperty.call(Object(object), property)
    },
    configurable: true,
    enumerable: false,
    writable: true,
  })
}
