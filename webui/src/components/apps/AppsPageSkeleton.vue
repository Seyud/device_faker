<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <section class="apps-page-skeleton" aria-hidden="true">
    <div class="apps-page-skeleton__header">
      <div class="apps-page-skeleton__title-row">
        <div class="apps-page-skeleton__title skeleton-loader"></div>
        <div class="apps-page-skeleton__button skeleton-loader"></div>
      </div>
      <div class="apps-page-skeleton__search skeleton-loader"></div>
    </div>

    <div class="apps-page-skeleton__tabs">
      <div class="apps-page-skeleton__tab skeleton-loader"></div>
      <div class="apps-page-skeleton__tab skeleton-loader"></div>
    </div>

    <div class="apps-page-skeleton__list">
      <div v-for="i in 12" :key="i" class="apps-page-skeleton__card glass-effect">
        <div class="apps-page-skeleton__icon skeleton-loader"></div>
        <div class="apps-page-skeleton__content">
          <div class="apps-page-skeleton__name skeleton-loader"></div>
          <div class="apps-page-skeleton__package skeleton-loader"></div>
          <div class="apps-page-skeleton__badges">
            <div class="apps-page-skeleton__badge skeleton-loader"></div>
            <div
              class="apps-page-skeleton__badge apps-page-skeleton__badge--short skeleton-loader"
            ></div>
          </div>
        </div>
        <div class="apps-page-skeleton__chevron skeleton-loader"></div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.apps-page-skeleton {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.apps-page-skeleton__header {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-bottom: 0.5rem;
}

.apps-page-skeleton__title-row {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  width: 100%;
}

.apps-page-skeleton__title {
  flex: 1 1 auto;
  min-width: 0;
  width: 8rem;
  height: 2rem;
  border-radius: 0.75rem;
}

.apps-page-skeleton__button {
  flex: 0 1 11rem;
  width: 11rem;
  height: 2rem;
  border-radius: 0.75rem;
}

.apps-page-skeleton__search {
  width: min(100%, 25rem);
  height: 2.5rem;
  border-radius: 0.9rem;
}

.apps-page-skeleton__tabs {
  display: flex;
  gap: 0.5rem;
  padding-bottom: 0.5rem;
}

.apps-page-skeleton__tab {
  flex: 1;
  height: 2.5rem;
  border-radius: 0.75rem;
}

.apps-page-skeleton__list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.apps-page-skeleton__card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-radius: 0.75rem;
  box-shadow: 0 2px 8px var(--shadow);
  pointer-events: none;
}

.apps-page-skeleton__icon {
  width: 3rem;
  height: 3rem;
  flex-shrink: 0;
  border-radius: 0.75rem;
}

.apps-page-skeleton__content {
  flex: 1;
  min-width: 0;
}

.apps-page-skeleton__name {
  width: min(60%, 12rem);
  height: 1rem;
  margin-bottom: 0.5rem;
  border-radius: 0.4rem;
}

.apps-page-skeleton__package {
  width: min(42%, 9rem);
  height: 0.75rem;
  margin-bottom: 0.5rem;
  border-radius: 0.4rem;
}

.apps-page-skeleton__badges {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.apps-page-skeleton__badge {
  width: 5rem;
  height: 1.25rem;
  border-radius: 0.35rem;
}

.apps-page-skeleton__badge--short {
  width: 3rem;
}

.apps-page-skeleton__chevron {
  width: 1rem;
  height: 1rem;
  border-radius: 999px;
  flex-shrink: 0;
}

.skeleton-loader {
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite linear;
  opacity: 0.7;
}

@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.dark .skeleton-loader {
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.05) 25%,
    rgba(255, 255, 255, 0.1) 50%,
    rgba(255, 255, 255, 0.05) 75%
  );
}
</style>
