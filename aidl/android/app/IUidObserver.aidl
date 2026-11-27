// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

package android.app;

oneway interface IUidObserver {
    void onUidGone(int uid, boolean disabled);
    void onUidActive(int uid);
    void onUidIdle(int uid, boolean disabled);
    void onUidStateChanged(int uid, int procState, long procStateSeq, int capability);
    void onUidProcAdjChanged(int uid, int adj);
    void onUidCachedChanged(int uid, boolean cached);
}
