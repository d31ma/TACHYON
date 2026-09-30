import type { ShopCounterElement } from '/.tachyon/elements/shop-counter.js';

const apples: ShopCounterElement | null = document.querySelector('shop-counter');

apples?.addEventListener('count-changed', (event) => {
  const count: number = event.detail.count;
  console.log(`Apples: ${count}`);
});

apples?.addEventListener('count-changed', (event) => {
  const count: string = event.detail.count;
  console.log(count);
});
