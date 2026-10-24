export default defineNuxtConfig({
  compatibilityDate: '2026-10-24',
  ssr: false,
  devtools: { enabled: false },
  telemetry: false,
  css: ['~/assets/main.css'],
  app: {
    head: {
      title: '재료 관리 | 순창한상',
      htmlAttrs: { lang: 'ko' },
      meta: [
        { name: 'description', content: '순창한상의 재료 이름과 관리 단위를 등록하고 확인합니다.' },
        { name: 'color-scheme', content: 'light dark' },
      ],
    },
  },
  sourcemap: { server: false, client: false },
})
