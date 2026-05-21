import { createApp } from "vue";
import { createPinia } from "pinia";
import { Quasar, Notify, Dialog, Loading } from "quasar";
import "@quasar/extras/material-icons/material-icons.css";
import "@quasar/extras/material-icons-outlined/material-icons-outlined.css";
import "quasar/src/css/index.sass";
import "@/styles/main.css";
import router from "@/router";
import App from "./App.vue";

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(Quasar, {
  plugins: { Notify, Dialog, Loading },
  config: {
    notify: {
      position: "top-right",
      timeout: 3000,
    },
  },
});

app.mount("#app");
