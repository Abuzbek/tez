import { mount } from "@tez/runtime-dom";
import { Counter } from "./counter";
import { TodoList } from "./todo-list";

mount(Counter, document.getElementById("counter-demo")!);
mount(TodoList, document.getElementById("todo-demo")!);
