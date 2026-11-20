import { boot } from "quasar/wrappers";
import { h, defineComponent, type PropType } from "vue";
import { Icon as IconifyIcon, addCollection } from "@iconify/vue";
import mingcute from "@iconify-json/mingcute/icons.json";
import simpleIcons from "@iconify-json/simple-icons/icons.json";
import gravityUi from "@iconify-json/gravity-ui/icons.json";
import maki from "@iconify-json/maki/icons.json";

// api.iconify.design может быть недоступен (или медленный), из-за чего иконки
// не отрисовывались. Регистрируем используемые наборы локально.
type Collection = Parameters<typeof addCollection>[0];
addCollection(mingcute as Collection);
addCollection(simpleIcons as Collection);
addCollection(gravityUi as Collection);
addCollection(maki as Collection);

const GlobalIcon = defineComponent({
  name: "AppIcon",
  props: {
    name: { type: String as PropType<string>, required: true },
    size: { type: [Number, String] as PropType<number | string>, default: 20 },
  },
  setup(props) {
    return () =>
      h(IconifyIcon, {
        icon: props.name,
        width: props.size,
        height: props.size,
        class: "ic",
      });
  },
});

declare module "vue" {
  interface GlobalComponents {
    Icon: typeof GlobalIcon;
  }
}

export default boot(({ app }) => {
  app.component("Icon", GlobalIcon);
});
