use std::borrow::Cow;
use std::sync::Arc;

use iced::Executor as _;
use iced_wgpu::graphics::{Shell, Viewport};
use iced_wgpu::{Engine, Renderer, wgpu};
use iced_winit::conversion;
use iced_winit::core::event;
use iced_winit::core::input_method::InputMethod;
use iced_winit::core::mouse;
use iced_winit::core::renderer;
use iced_winit::core::shell;
use iced_winit::core::time::Instant;
use iced_winit::core::window;
use iced_winit::core::{Event, Size};
use iced_winit::futures::Runtime;
use iced_winit::futures::subscription;
use iced_winit::runtime::user_interface::{self, UserInterface};
use iced_winit::runtime::{self, Action, Task};
use iced_winit::winit;
use iced_winit::{Clipboard, Proxy};
use kovia_atlas::hybrid::{AtlasEngine, VIEW_FORMAT};
use kovia_protocol::{MapInput, PointerButton};
use kovia_ui::{Kovia, MapViewport, Message, PHOSPHOR_FONT_BYTES, TX02_FONT_BYTES};
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;

const SIDEBAR_LOGICAL: f64 = 244.0;
const INSPECTOR_LOGICAL: f64 = 354.0;

type UiRuntime = Runtime<iced::executor::Default, Proxy<Message>, Action<Message>>;

struct MapTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
}

impl MapTarget {
    fn new(device: &wgpu::Device, size: [u32; 2]) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Kovia shared Atlas target"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: VIEW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            size,
        }
    }
}

fn map_target_size(window: &winit::window::Window) -> Option<[u32; 2]> {
    let size = window.inner_size();
    map_target_size_for([size.width, size.height], window.scale_factor())
}

fn map_target_size_for(window_size: [u32; 2], scale_factor: f64) -> Option<[u32; 2]> {
    let reserved = ((SIDEBAR_LOGICAL + INSPECTOR_LOGICAL) * scale_factor) as u32;
    let width = window_size[0].saturating_sub(reserved);
    (width > 0 && window_size[1] > 0).then_some([width, window_size[1]])
}

#[allow(clippy::large_enum_variant)]
enum RunnerState {
    Loading,
    Ready(Box<ReadyState>),
    Exiting,
}

struct Runner {
    state: RunnerState,
    runtime: UiRuntime,
    proxy: Proxy<Message>,
}

struct ReadyState {
    surface_format: wgpu::TextureFormat,
    iced_window_id: window::Id,
    iced_events: Vec<Event>,
    cursor: mouse::Cursor,
    cache: user_interface::Cache,
    viewport: Viewport,
    modifiers: ModifiersState,
    resized: bool,
    suspended: bool,
    clipboard: Clipboard,
    // Rust drops fields in declaration order. Keep the consumers ahead of the
    // shared target and shell-owned GPU handles.
    iced_renderer: Renderer,
    ui: Kovia,
    atlas: AtlasEngine,
    map_target: MapTarget,
    surface: wgpu::Surface<'static>,
    _queue: wgpu::Queue,
    device: wgpu::Device,
    _adapter: wgpu::Adapter,
    _instance: wgpu::Instance,
    window: Arc<winit::window::Window>,
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::<Action<Message>>::with_user_event().build()?;
    let (proxy, worker) = Proxy::new(event_loop.create_proxy());
    let executor = iced::executor::Default::new().expect("create Iced executor");
    executor.spawn(worker);
    let runtime = Runtime::new(executor, proxy.clone());
    let mut runner = Runner {
        state: RunnerState::Loading,
        runtime,
        proxy,
    };
    event_loop.run_app(&mut runner)
}

impl winit::application::ApplicationHandler<Action<Message>> for Runner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if !matches!(self.state, RunnerState::Loading) {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kovia — Worldbuilding Tool")
                        .with_inner_size(winit::dpi::LogicalSize::new(1440.0, 900.0)),
                )
                .expect("create Kovia window"),
        );
        let physical_size = window.inner_size();
        let viewport = iced_viewport(&window);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or_default(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("create presentation surface");
        let (surface_format, adapter, device, queue) = self.runtime.block_on(async {
            let adapter =
                wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface))
                    .await
                    .expect("request shared adapter");
            let capabilities = surface.get_capabilities(&adapter);
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .or_else(|| capabilities.formats.first().copied())
                .expect("surface format");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("Kovia shell-owned device"),
                    required_features: adapter.features(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    trace: wgpu::Trace::Off,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                })
                .await
                .expect("request shared device");
            (format, adapter, device, queue)
        });
        configure_surface(&surface, &device, surface_format, physical_size);

        load_ui_fonts();
        let target_size = map_target_size(&window).expect("initial Atlas viewport is non-zero");
        let map_target = MapTarget::new(&device, target_size);
        let atlas = AtlasEngine::new_demo(
            &instance,
            &adapter,
            &device,
            &queue,
            map_target.view.clone(),
            target_size,
        );
        let (mut ui, boot_task) = Kovia::boot(MapViewport::new(
            map_target.view.clone(),
            target_size,
            window.scale_factor() as f32,
        ));
        ui.set_map_snapshot(atlas.snapshot());

        let iced_renderer = Renderer::new(
            Engine::new(
                &adapter,
                device.clone(),
                queue.clone(),
                surface_format,
                None,
                Shell::new(self.proxy.clone()),
            ),
            renderer::Settings {
                default_font: Kovia::default_font(),
                ..renderer::Settings::default()
            },
        );

        run_task(&mut self.runtime, boot_task);
        track_subscriptions(&mut self.runtime, &ui);
        eprintln!(
            "Kovia shared adapter={:?}; Atlas target={target_size:?}; surface={surface_format:?}",
            adapter.get_info(),
        );
        event_loop.set_control_flow(ControlFlow::Wait);
        window.request_redraw();
        self.state = RunnerState::Ready(Box::new(ReadyState {
            surface_format,
            iced_window_id: window::Id::unique(),
            iced_events: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            cache: user_interface::Cache::new(),
            viewport,
            modifiers: ModifiersState::default(),
            resized: false,
            suspended: false,
            clipboard: Clipboard::new(),
            iced_renderer,
            ui,
            atlas,
            map_target,
            surface,
            _queue: queue,
            device,
            _adapter: adapter,
            _instance: instance,
            window,
        }));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if matches!(&event, WindowEvent::CloseRequested) {
            let previous = std::mem::replace(&mut self.state, RunnerState::Exiting);
            if let RunnerState::Ready(state) = previous {
                state.finish_gpu_work();
                drop(state);
            }
            event_loop.exit();
            return;
        }
        let RunnerState::Ready(state) = &mut self.state else {
            return;
        };

        if let WindowEvent::MouseInput {
            state: winit::event::ElementState::Released,
            button,
            ..
        } = &event
        {
            state.release_map_button(*button);
        }

        match event {
            WindowEvent::RedrawRequested => state.redraw(&mut self.runtime, &self.proxy),
            WindowEvent::CursorMoved { position, .. } => {
                state.cursor = mouse::Cursor::Available(conversion::cursor_position(
                    position,
                    state.viewport.scale_factor(),
                ));
            }
            WindowEvent::CursorEntered { .. } => {
                state.atlas.send_input(MapInput::FocusChanged(true));
            }
            WindowEvent::CursorLeft { .. } => {
                state.cursor = mouse::Cursor::Unavailable;
                state.atlas.send_input(MapInput::FocusChanged(false));
            }
            WindowEvent::ModifiersChanged(modifiers) => state.modifiers = modifiers.state(),
            WindowEvent::Focused(focused) => {
                state.atlas.send_input(MapInput::FocusChanged(focused));
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                state.resized = true;
                state.window.request_redraw();
            }
            _ => {}
        }

        if let Some(iced_event) =
            conversion::window_event(event, state.window.scale_factor() as f32, state.modifiers)
        {
            state.iced_events.push(iced_event);
        }
        state.process_iced_events(&mut self.runtime, &self.proxy);
    }

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        action: Action<Message>,
    ) {
        self.proxy.free_slots(1);
        let RunnerState::Ready(state) = &mut self.state else {
            return;
        };
        match action {
            Action::Output(message) => {
                state.dispatch_message(message, &mut self.runtime);
            }
            Action::Widget(mut operation) => {
                state.operate(&mut *operation);
                state.window.request_redraw();
            }
            Action::Clipboard(action) => match action {
                runtime::clipboard::Action::Read { kind, channel } => {
                    state.clipboard.read(kind, move |result| {
                        let _ = channel.send(result);
                    });
                }
                runtime::clipboard::Action::Write { content, channel } => {
                    state.clipboard.write(content, move |result| {
                        let _ = channel.send(result);
                    });
                }
            },
            Action::Window(runtime::window::Action::RedrawAll) => state.window.request_redraw(),
            Action::Window(runtime::window::Action::RelayoutAll) => {
                state.cache = user_interface::Cache::new();
                state.window.request_redraw();
            }
            Action::Event { window: _, event } => {
                state.iced_events.push(event);
                state.process_iced_events(&mut self.runtime, &self.proxy);
            }
            Action::Tick | Action::Reload => state.window.request_redraw(),
            Action::Exit => {
                event_loop.exit();
            }
            Action::Window(_)
            | Action::System(_)
            | Action::Font(_)
            | Action::Image(_)
            | Action::Backend(_) => {}
        }
    }
}

impl ReadyState {
    fn release_map_button(&mut self, button: winit::event::MouseButton) {
        let button = match button {
            winit::event::MouseButton::Left => PointerButton::Left,
            winit::event::MouseButton::Right => PointerButton::Right,
            winit::event::MouseButton::Middle => PointerButton::Middle,
            winit::event::MouseButton::Back
            | winit::event::MouseButton::Forward
            | winit::event::MouseButton::Other(_) => return,
        };
        let scale = self.window.scale_factor() as f32;
        let position = self.cursor.position().unwrap_or_default();
        let physical_position = [
            ((position.x - SIDEBAR_LOGICAL as f32) * scale)
                .clamp(0.0, self.map_target.size[0].saturating_sub(1) as f32),
            (position.y * scale).clamp(0.0, self.map_target.size[1].saturating_sub(1) as f32),
        ];
        self.atlas.send_input(MapInput::PointerUp {
            physical_position,
            button,
        });
    }

    fn finish_gpu_work(&self) {
        let result = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(5)),
        });
        eprintln!("Kovia shutdown GPU drain: {result:?}");
    }

    fn resize_if_needed(&mut self) {
        if !self.resized {
            return;
        }
        self.viewport = iced_viewport(&self.window);
        let physical_size = self.window.inner_size();
        let Some(map_size) = map_target_size(&self.window) else {
            self.suspended = true;
            self.resized = false;
            return;
        };
        configure_surface(
            &self.surface,
            &self.device,
            self.surface_format,
            physical_size,
        );
        let target = MapTarget::new(&self.device, map_size);
        self.atlas.replace_target(target.view.clone(), map_size);
        self.atlas.send_input(MapInput::Resize {
            physical_size: map_size,
            scale_factor: self.window.scale_factor(),
        });
        self.ui.replace_viewport(
            target.view.clone(),
            map_size,
            self.window.scale_factor() as f32,
        );
        self.map_target = target;
        self.cache = user_interface::Cache::new();
        self.suspended = false;
        self.resized = false;
        eprintln!("Recreated shared Atlas target: {map_size:?}");
    }

    fn redraw(&mut self, runtime: &mut UiRuntime, proxy: &Proxy<Message>) {
        self.resize_if_needed();
        if self.suspended {
            return;
        }

        self.apply_atlas_update();
        let wgpu::CurrentSurfaceTexture::Success(frame) = self.surface.get_current_texture() else {
            self.window.request_redraw();
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut interface = UserInterface::build(
            self.ui.view(),
            self.viewport.logical_size(),
            std::mem::take(&mut self.cache),
            &mut self.iced_renderer,
        );
        let redraw_event = Event::Window(window::Event::RedrawRequested(Instant::now()));
        let waker = ui_waker(proxy.clone());
        let mut messages = shell::Bus::new();
        let (state, statuses) = interface.update(
            &*self.window,
            &waker,
            std::slice::from_ref(&redraw_event),
            self.cursor,
            &mut self.iced_renderer,
            &mut messages,
        );
        runtime.broadcast(subscription::Event::Interaction {
            window: self.iced_window_id,
            event: redraw_event,
            status: statuses
                .into_iter()
                .next()
                .unwrap_or(event::Status::Ignored),
        });
        interface.draw(
            &mut self.iced_renderer,
            &self.ui.theme(),
            &renderer::Style {
                text_color: self.ui.theme().palette().background.base.text,
            },
            self.cursor,
        );
        self.cache = interface.into_cache();
        self.handle_ui_state(state, proxy);
        for message in messages.drain() {
            self.dispatch_message(message, runtime);
        }
        self.iced_renderer.present(
            Some(iced_winit::core::Color::from_rgb8(9, 11, 13)),
            frame.texture.format(),
            &view,
            &self.viewport,
        );
        self.window.pre_present_notify();
        frame.present();
    }

    fn process_iced_events(&mut self, runtime: &mut UiRuntime, proxy: &Proxy<Message>) {
        if self.iced_events.is_empty() || self.suspended {
            return;
        }
        let events = std::mem::take(&mut self.iced_events);
        let mut interface = UserInterface::build(
            self.ui.view(),
            self.viewport.logical_size(),
            std::mem::take(&mut self.cache),
            &mut self.iced_renderer,
        );
        let mut messages = shell::Bus::new();
        let (state, statuses) = interface.update(
            &*self.window,
            &ui_waker(proxy.clone()),
            &events,
            self.cursor,
            &mut self.iced_renderer,
            &mut messages,
        );
        for (event, status) in events.into_iter().zip(statuses) {
            runtime.broadcast(subscription::Event::Interaction {
                window: self.iced_window_id,
                event,
                status,
            });
        }
        self.cache = interface.into_cache();
        self.handle_ui_state(state, proxy);
        for message in messages.drain() {
            self.dispatch_message(message, runtime);
        }
        self.window.request_redraw();
    }

    fn dispatch_message(&mut self, message: Message, runtime: &mut UiRuntime) {
        match &message {
            Message::MapInput(input) => self.atlas.send_input(*input),
            Message::MapCommand(command) => self.atlas.send_command(command.clone()),
            _ => {}
        }
        let task = self.ui.update(message);
        run_task(runtime, task);
        track_subscriptions(runtime, &self.ui);
        self.window.request_redraw();
    }

    fn apply_atlas_update(&mut self) {
        for event in self.atlas.update() {
            self.ui.apply_map_event(&event);
        }
        self.ui.set_map_snapshot(self.atlas.snapshot());
    }

    fn handle_ui_state(&mut self, state: user_interface::State, proxy: &Proxy<Message>) {
        if let user_interface::State::Updated {
            mouse_interaction,
            redraw_request,
            input_method,
            clipboard,
            ..
        } = state
        {
            if let Some(cursor) = conversion::mouse_interaction(mouse_interaction) {
                self.window.set_cursor(cursor);
            }
            if !matches!(redraw_request, window::RedrawRequest::Wait) {
                self.window.request_redraw();
            }
            apply_input_method(&self.window, input_method);
            run_clipboard(proxy, &mut self.clipboard, clipboard, self.iced_window_id);
        }
    }

    fn operate(&mut self, operation: &mut dyn iced_winit::core::widget::Operation) {
        let mut interface = UserInterface::build(
            self.ui.view(),
            self.viewport.logical_size(),
            std::mem::take(&mut self.cache),
            &mut self.iced_renderer,
        );
        interface.operate(&self.iced_renderer, operation);
        self.cache = interface.into_cache();
    }
}

fn run_task(runtime: &mut UiRuntime, task: Task<Message>) {
    if let Some(stream) = runtime::task::into_stream(task) {
        runtime.run(stream);
    }
}

fn track_subscriptions(runtime: &mut UiRuntime, ui: &Kovia) {
    runtime.track(subscription::into_recipes(
        runtime.enter(|| ui.subscription().map(Action::Output)),
    ));
}

fn ui_waker(proxy: Proxy<Message>) -> shell::Waker {
    shell::Waker::new(move || proxy.send_action(Action::Tick))
}

fn run_clipboard(
    proxy: &Proxy<Message>,
    clipboard: &mut Clipboard,
    requests: iced_winit::core::Clipboard,
    window: window::Id,
) {
    for kind in requests.reads {
        let proxy = proxy.clone();
        clipboard.read(kind, move |result| {
            proxy.send_action(Action::Event {
                window,
                event: Event::Clipboard(iced_winit::core::clipboard::Event::Read(
                    result.map(Arc::new),
                )),
            });
        });
    }
    if let Some(content) = requests.write {
        let proxy = proxy.clone();
        clipboard.write(content, move |result| {
            proxy.send_action(Action::Event {
                window,
                event: Event::Clipboard(iced_winit::core::clipboard::Event::Written(result)),
            });
        });
    }
}

fn apply_input_method(window: &winit::window::Window, input_method: InputMethod) {
    match input_method {
        InputMethod::Disabled => window.set_ime_allowed(false),
        InputMethod::Enabled {
            cursor, purpose, ..
        } => {
            window.set_ime_allowed(true);
            window.set_ime_cursor_area(
                winit::dpi::LogicalPosition::new(cursor.x, cursor.y),
                winit::dpi::LogicalSize::new(cursor.width, cursor.height),
            );
            window.set_ime_purpose(conversion::ime_purpose(purpose));
        }
    }
}

fn load_ui_fonts() {
    let mut fonts = iced_wgpu::graphics::text::font_system()
        .write()
        .expect("write Iced font system");
    fonts.load_font(Cow::Borrowed(PHOSPHOR_FONT_BYTES));
    fonts.load_font(Cow::Borrowed(TX02_FONT_BYTES));
}

fn iced_viewport(window: &winit::window::Window) -> Viewport {
    let size = window.inner_size();
    Viewport::with_physical_size(
        Size::new(size.width, size.height),
        renderer::Scale {
            window: window.scale_factor() as f32,
            application: 1.0,
        },
    )
}

fn configure_surface(
    surface: &wgpu::Surface<'_>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: winit::dpi::PhysicalSize<u32>,
) {
    if size.width == 0 || size.height == 0 {
        return;
    }
    surface.configure(
        device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sized_or_panel_only_windows_suspend_the_map() {
        assert_eq!(map_target_size_for([1440, 900], 1.0), Some([842, 900]));
        assert_eq!(map_target_size_for([1440, 0], 1.0), None);
        assert_eq!(map_target_size_for([598, 900], 1.0), None);
        assert_eq!(map_target_size_for([1000, 900], 2.0), None);
    }
}
