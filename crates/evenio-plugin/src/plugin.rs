use evenio::world::World;

/// Simplified bevy plugin for evenio use
pub trait Plugin: Send + Sync {
    /// Configures the [`World`] to which this plugin is added.
    fn build(&self, world: &mut World);
}

impl<T: Fn(&mut World) + Send + Sync + 'static> Plugin for T {
    fn build(&self, world: &mut World) {
        self(world);
    }
}

pub trait WorldPluginExt {
    fn add_plugin<P: Plugin>(&mut self, plugin: P);
}

impl WorldPluginExt for World {
    fn add_plugin<P: Plugin>(&mut self, plugin: P) {
        plugin.build(self);
    }
}
