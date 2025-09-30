use std::sync::{Arc, Condvar, Mutex};
use enumset::enum_set;

use esp_idf_svc::bt::ble::gap::{AdvConfiguration, BleGapEvent, EspBleGap};
use esp_idf_svc::bt::ble::gatt::server::{ConnectionId, EspGatts, GattsEvent, TransferId};
use esp_idf_svc::bt::ble::gatt::{
    AutoResponse, GattCharacteristic, GattDescriptor, GattId, GattInterface, GattResponse,
    GattServiceId, GattStatus, Handle, Permission, Property,
};
use esp_idf_svc::bt::{BdAddr, Ble, BtDriver, BtStatus, BtUuid};
use esp_idf_svc::sys::{EspError, ESP_FAIL};

use log::{info, warn};

// 我们的服务UUID
pub const SERVICE_UUID: u128 = 0xad91b201734740479e173bed82d75f9d;

/// 我们的"recv"特征 - 客户端可以发送数据的地方
pub const RECV_CHARACTERISTIC_UUID: u128 = 0xb6fccb5087be44f3ae22f85485ea42c4;
/// 我们的"indicate"特征 - 客户端可以接收数据的地方
pub const IND_CHARACTERISTIC_UUID: u128 = 0x503de214868246c4828fd59144da41be;

const APP_ID: u16 = 0;
const MAX_CONNECTIONS: usize = 2;

#[derive(Debug, Clone)]
struct Connection {
    peer: BdAddr,
    conn_id: Handle,
    subscribed: bool,
    mtu: Option<u16>,
}

#[derive(Default)]
struct State {
    gatt_if: Option<GattInterface>,
    service_handle: Option<Handle>,
    recv_handle: Option<Handle>,
    ind_handle: Option<Handle>,
    ind_cccd_handle: Option<Handle>,
    connections: heapless::Vec<Connection, MAX_CONNECTIONS>,
    response: GattResponse,
    ind_confirmed: Option<BdAddr>,
}

pub struct BluetoothManager {
    gap: Arc<EspBleGap<'static, Ble, Arc<BtDriver<'static, Ble>>>>,
    gatts: Arc<EspGatts<'static, Ble, Arc<BtDriver<'static, Ble>>>>,
    state: Arc<Mutex<State>>,
    condvar: Arc<Condvar>,
    is_connected: Arc<Mutex<bool>>,
    received_data: Arc<Mutex<Vec<u8>>>,
}

impl BluetoothManager {
    pub fn new(
        gap: Arc<EspBleGap<'static, Ble, Arc<BtDriver<'static, Ble>>>>,
        gatts: Arc<EspGatts<'static, Ble, Arc<BtDriver<'static, Ble>>>>,
    ) -> Self {
        Self {
            gap,
            gatts,
            state: Arc::new(Mutex::new(Default::default())),
            condvar: Arc::new(Condvar::new()),
            is_connected: Arc::new(Mutex::new(false)),
            received_data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("初始化BLE GATT服务器...");
        
        let gap_server = self.clone();
        self.gap.subscribe(move |event| {
            gap_server.check_esp_status(gap_server.on_gap_event(event));
        })?;

        let gatts_server = self.clone();
        self.gatts.subscribe(move |(gatt_if, event)| {
            gatts_server.check_esp_status(gatts_server.on_gatts_event(gatt_if, event))
        })?;

        info!("BLE Gap和Gatts订阅初始化完成");

        self.gatts.register_app(APP_ID)?;
        info!("Gatts BTP应用已注册");

        Ok(())
    }

    /// GAP事件处理器
    fn on_gap_event(&self, event: BleGapEvent) -> Result<(), EspError> {
        info!("收到GAP事件: {event:?}");

        match event {
            BleGapEvent::AdvertisingConfigured(status) => {
                if let Err(e) = self.check_bt_status(status) {
                    warn!("广播配置状态错误: {:?}", e);
                    return Err(e);
                }
                if let Err(e) = self.gap.start_advertising() {
                    warn!("开始广播失败: {:?}", e);
                    return Err(e);
                }
            }
            BleGapEvent::AdvertisingStarted(status) => {
                if let Err(e) = self.check_bt_status(status) {
                    warn!("广播启动状态错误: {:?}", e);
                    return Err(e);
                }
                info!("BLE广播已开始");
            }
            BleGapEvent::AdvertisingStopped(status) => {
                warn!("广播已停止: {:?}", status);
                // 广播停止后，尝试重新开始广播
                if let Err(e) = self.gap.start_advertising() {
                    warn!("重新开始广播失败: {:?}", e);
                    return Err(e);
                }
            }
            _ => {
                // 其他事件正常处理
            }
        }

        Ok(())
    }

    /// GATTS事件处理器
    fn on_gatts_event(&self, gatt_if: GattInterface, event: GattsEvent) -> Result<(), EspError> {
        info!("收到GATTS事件: {event:?}");

        match event {
            GattsEvent::ServiceRegistered { status, app_id } => {
                if let Err(e) = self.check_gatt_status(status) {
                    warn!("服务注册状态错误: {:?}", e);
                    return Err(e);
                }
                if APP_ID == app_id {
                    if let Err(e) = self.create_service(gatt_if) {
                        warn!("创建服务失败: {:?}", e);
                        return Err(e);
                    }
                }
            }
            GattsEvent::ServiceCreated {
                status,
                service_handle,
                ..
            } => {
                if let Err(e) = self.check_gatt_status(status) {
                    warn!("服务创建状态错误: {:?}", e);
                    return Err(e);
                }
                if let Err(e) = self.configure_and_start_service(service_handle) {
                    warn!("配置服务失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::CharacteristicAdded {
                status,
                attr_handle,
                service_handle,
                char_uuid,
            } => {
                if let Err(e) = self.check_gatt_status(status) {
                    warn!("特征添加状态错误: {:?}", e);
                    return Err(e);
                }
                if let Err(e) = self.register_characteristic(service_handle, attr_handle, char_uuid) {
                    warn!("注册特征失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::DescriptorAdded {
                status,
                attr_handle,
                service_handle,
                descr_uuid,
            } => {
                if let Err(e) = self.check_gatt_status(status) {
                    warn!("描述符添加状态错误: {:?}", e);
                    return Err(e);
                }
                if let Err(e) = self.register_cccd_descriptor(service_handle, attr_handle, descr_uuid) {
                    warn!("注册CCCD描述符失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::Mtu { conn_id, mtu } => {
                if let Err(e) = self.register_conn_mtu(conn_id, mtu) {
                    warn!("注册连接MTU失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::PeerConnected { conn_id, addr, .. } => {
                if let Err(e) = self.create_conn(conn_id, addr) {
                    warn!("创建连接失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::PeerDisconnected { addr, .. } => {
                if let Err(e) = self.delete_conn(addr) {
                    warn!("删除连接失败: {:?}", e);
                    return Err(e);
                }
            }
            GattsEvent::Read {
                conn_id,
                trans_id,
                addr,
                handle,
                offset,
                is_long: _,
                need_rsp: _,
            } => {
                // 处理读取请求
                info!("收到读取请求: conn_id={}, handle={}, offset={}, addr={}", conn_id, handle, offset, addr);
                
                // 检查是否是我们的特征值
                let state = self.state.lock().unwrap();
                if Some(handle) == state.recv_handle {
                    info!("客户端读取RECV特征值");
                    // 对于RECV特征值，返回空数据
                    self.gatts.send_response(
                        gatt_if,
                        conn_id,
                        trans_id,
                        GattStatus::Ok,
                        None,
                    )?;
                } else if Some(handle) == state.ind_handle {
                    info!("客户端读取IND特征值");
                    // 对于IND特征值，返回空数据
                    self.gatts.send_response(
                        gatt_if,
                        conn_id,
                        trans_id,
                        GattStatus::Ok,
                        None,
                    )?;
                } else if Some(handle) == state.ind_cccd_handle {
                    info!("客户端读取CCCD描述符");
                    // 对于CCCD描述符，需要返回具体的值
                    let mut response = GattResponse::new();
                    response.attr_handle(handle)
                        .auth_req(0)
                        .offset(offset)
                        .value(&[0x00, 0x00])  // CCCD默认值：未订阅
                        .map_err(|_| EspError::from_infallible::<ESP_FAIL>())?;
                    
                    self.gatts.send_response(
                        gatt_if,
                        conn_id,
                        trans_id,
                        GattStatus::Ok,
                        Some(&response),
                    )?;
                } else {
                    info!("客户端读取未知特征值: handle={}", handle);
                    // 对于未知特征值，返回错误
                    self.gatts.send_response(
                        gatt_if,
                        conn_id,
                        trans_id,
                        GattStatus::Error,
                        None,
                    )?;
                }
            }
            GattsEvent::Write {
                conn_id,
                trans_id,
                addr,
                handle,
                offset,
                need_rsp,
                is_prep,
                value,
            } => {
                info!("收到写入请求: conn_id={}, handle={}, offset={}, addr={}, value_len={}", 
                      conn_id, handle, offset, addr, value.len());
                info!("写入数据内容: {:?}", value);
                
                match self.recv(
                    gatt_if, conn_id, trans_id, addr, handle, offset, need_rsp, is_prep, value,
                ) {
                    Ok(handled) => {
                        if handled {
                            if let Err(e) = self.send_write_response(
                                gatt_if, conn_id, trans_id, handle, offset, need_rsp, is_prep, value,
                            ) {
                                warn!("发送写入响应失败: {:?}", e);
                                return Err(e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("处理写入数据失败: {:?}", e);
                        return Err(e);
                    }
                }
            }
            GattsEvent::Confirm { status, .. } => {
                if let Err(e) = self.check_gatt_status(status) {
                    warn!("确认状态错误: {:?}", e);
                    return Err(e);
                }
                if let Err(e) = self.confirm_indication() {
                    warn!("确认指示失败: {:?}", e);
                    return Err(e);
                }
            }
            _ => (),
        }

        Ok(())
    }

    /// 创建服务并开始广播
    fn create_service(&self, gatt_if: GattInterface) -> Result<(), EspError> {
        self.state.lock().unwrap().gatt_if = Some(gatt_if);

        self.gap.set_device_name("ESP32-IR-Recorder")?;
        self.gap.set_adv_conf(&AdvConfiguration {
            include_name: true,
            include_txpower: true,
            flag: 2,
            service_uuid: Some(BtUuid::uuid128(SERVICE_UUID)),
            ..Default::default()
        })?;
        self.gatts.create_service(
            gatt_if,
            &GattServiceId {
                id: GattId {
                    uuid: BtUuid::uuid128(SERVICE_UUID),
                    inst_id: 0,
                },
                is_primary: true,
            },
            8,
        )?;

        Ok(())
    }

    /// 配置并启动服务
    fn configure_and_start_service(&self, service_handle: Handle) -> Result<(), EspError> {
        self.state.lock().unwrap().service_handle = Some(service_handle);

        self.gatts.start_service(service_handle)?;
        self.add_characteristics(service_handle)?;

        Ok(())
    }

    /// 添加特征到服务
    fn add_characteristics(&self, service_handle: Handle) -> Result<(), EspError> {
        self.gatts.add_characteristic(
            service_handle,
            &GattCharacteristic {
                uuid: BtUuid::uuid128(RECV_CHARACTERISTIC_UUID),
                permissions: enum_set!(Permission::Write | Permission::Read),
                properties: enum_set!(Property::Write | Property::Read),
                max_len: 200, // 最大接收数据
                auto_rsp: AutoResponse::ByGatt,
            },
            &[],
        )?;

        self.gatts.add_characteristic(
            service_handle,
            &GattCharacteristic {
                uuid: BtUuid::uuid128(IND_CHARACTERISTIC_UUID),
                permissions: enum_set!(Permission::Write | Permission::Read),
                properties: enum_set!(Property::Indicate | Property::Read),
                max_len: 200, // 最大指示数据
                auto_rsp: AutoResponse::ByGatt,
            },
            &[],
        )?;

        Ok(())
    }

    /// 注册特征
    fn register_characteristic(
        &self,
        service_handle: Handle,
        attr_handle: Handle,
        char_uuid: BtUuid,
    ) -> Result<(), EspError> {
        let indicate_char = {
            let mut state = self.state.lock().unwrap();

            if state.service_handle != Some(service_handle) {
                false
            } else if char_uuid == BtUuid::uuid128(RECV_CHARACTERISTIC_UUID) {
                state.recv_handle = Some(attr_handle);
                false
            } else if char_uuid == BtUuid::uuid128(IND_CHARACTERISTIC_UUID) {
                state.ind_handle = Some(attr_handle);
                true
            } else {
                false
            }
        };

        if indicate_char {
            self.gatts.add_descriptor(
                service_handle,
                &GattDescriptor {
                    uuid: BtUuid::uuid16(0x2902), // CCCD
                    permissions: enum_set!(Permission::Read | Permission::Write),
                },
            )?;
        }

        Ok(())
    }

    /// 注册CCCD描述符
    fn register_cccd_descriptor(
        &self,
        service_handle: Handle,
        attr_handle: Handle,
        descr_uuid: BtUuid,
    ) -> Result<(), EspError> {
        let mut state = self.state.lock().unwrap();

        if descr_uuid == BtUuid::uuid16(0x2902) // CCCD UUID
            && state.service_handle == Some(service_handle)
        {
            state.ind_cccd_handle = Some(attr_handle);
        }

        Ok(())
    }

    /// 注册连接MTU
    fn register_conn_mtu(&self, conn_id: ConnectionId, mtu: u16) -> Result<(), EspError> {
        let mut state = self.state.lock().unwrap();

        if let Some(conn) = state
            .connections
            .iter_mut()
            .find(|conn| conn.conn_id == conn_id)
        {
            conn.mtu = Some(mtu);
        }

        Ok(())
    }

    /// 创建新连接
    fn create_conn(&self, conn_id: ConnectionId, addr: BdAddr) -> Result<(), EspError> {
        let added = {
            let mut state = self.state.lock().unwrap();

            if state.connections.len() < MAX_CONNECTIONS {
                state
                    .connections
                    .push(Connection {
                        peer: addr,
                        conn_id,
                        subscribed: false,
                        mtu: None,
                    })
                    .map_err(|_| ())
                    .unwrap();

                true
            } else {
                false
            }
        };

        if added {
            self.gap.set_conn_params_conf(addr, 10, 20, 0, 400)?;
            // 更新连接状态
            if let Ok(mut connected) = self.is_connected.lock() {
                *connected = true;
            }
            info!("BLE客户端连接: {}", addr);
        }

        Ok(())
    }

    /// 删除连接
    fn delete_conn(&self, addr: BdAddr) -> Result<(), EspError> {
        let mut state = self.state.lock().unwrap();

        if let Some(index) = state
            .connections
            .iter()
            .position(|Connection { peer, .. }| *peer == addr)
        {
            state.connections.swap_remove(index);
        }

        // 更新连接状态
        if let Ok(mut connected) = self.is_connected.lock() {
            *connected = false;
        }
        info!("BLE客户端断开连接: {}", addr);

        // 如果没有其他连接，重新开始广播
        if state.connections.is_empty() {
            info!("所有客户端已断开，重新开始广播...");
            // 重新开始广播
            if let Err(e) = self.gap.start_advertising() {
                warn!("重新开始广播失败: {:?}", e);
            }
        }

        Ok(())
    }

    /// 接收数据
    #[allow(clippy::too_many_arguments)]
    fn recv(
        &self,
        _gatt_if: GattInterface,
        conn_id: ConnectionId,
        _trans_id: TransferId,
        addr: BdAddr,
        handle: Handle,
        offset: u16,
        _need_rsp: bool,
        _is_prep: bool,
        value: &[u8],
    ) -> Result<bool, EspError> {
        let mut state = self.state.lock().unwrap();

        let recv_handle = state.recv_handle;
        let ind_cccd_handle = state.ind_cccd_handle;

        let Some(conn) = state
            .connections
            .iter_mut()
            .find(|conn| conn.conn_id == conn_id)
        else {
            return Ok(false);
        };

        if Some(handle) == ind_cccd_handle {
            // 订阅或取消订阅指示特征
            if offset == 0 && value.len() == 2 {
                let value = u16::from_le_bytes([value[0], value[1]]);
                if value == 0x02 {
                    if !conn.subscribed {
                        conn.subscribed = true;
                        info!("客户端 {} 订阅了指示特征", conn.peer);
                    }
                } else if conn.subscribed {
                    conn.subscribed = false;
                    info!("客户端 {} 取消订阅指示特征", conn.peer);
                }
            }
        } else if Some(handle) == recv_handle {
            // 在recv特征上接收数据
            info!("从 {} 接收数据: {:?}", addr, value);
            
            // 将数据添加到接收缓冲区
            if let Ok(mut data_vec) = self.received_data.lock() {
                data_vec.extend_from_slice(value);
            }
        } else {
            return Ok(false);
        }

        Ok(true)
    }

    /// 发送写入响应
    #[allow(clippy::too_many_arguments)]
    fn send_write_response(
        &self,
        gatt_if: GattInterface,
        conn_id: ConnectionId,
        trans_id: TransferId,
        handle: Handle,
        offset: u16,
        need_rsp: bool,
        is_prep: bool,
        value: &[u8],
    ) -> Result<(), EspError> {
        if !need_rsp {
            return Ok(());
        }

        if is_prep {
            let mut state = self.state.lock().unwrap();

            state
                .response
                .attr_handle(handle)
                .auth_req(0)
                .offset(offset)
                .value(value)
                .map_err(|_| EspError::from_infallible::<ESP_FAIL>())?;

            self.gatts.send_response(
                gatt_if,
                conn_id,
                trans_id,
                GattStatus::Ok,
                Some(&state.response),
            )?;
        } else {
            self.gatts
                .send_response(gatt_if, conn_id, trans_id, GattStatus::Ok, None)?;
        }

        Ok(())
    }

    /// 确认指示
    fn confirm_indication(&self) -> Result<(), EspError> {
        let mut state = self.state.lock().unwrap();
        if state.ind_confirmed.is_none() {
            unreachable!();
        }

        state.ind_confirmed = None;
        self.condvar.notify_all();

        Ok(())
    }

    /// 发送指示数据到所有订阅的客户端
    fn indicate(&self, data: &[u8]) -> Result<(), EspError> {
        for peer_index in 0..MAX_CONNECTIONS {
            let mut state = self.state.lock().unwrap();

            loop {
                if state.connections.len() <= peer_index {
                    break;
                }

                let Some(gatt_if) = state.gatt_if else {
                    break;
                };

                let Some(ind_handle) = state.ind_handle else {
                    break;
                };

                if state.ind_confirmed.is_none() {
                    let conn = &state.connections[peer_index];

                    self.gatts
                        .indicate(gatt_if, conn.conn_id, ind_handle, data)?;

                    state.ind_confirmed = Some(conn.peer);
                    let conn = &state.connections[peer_index];

                    info!("向 {} 发送指示数据", conn.peer);
                    break;
                } else {
                    state = self.condvar.wait(state).unwrap();
                }
            }
        }

        Ok(())
    }

    // 公共接口方法
    pub fn is_connected(&self) -> bool {
        if let Ok(connected) = self.is_connected.lock() {
            *connected
        } else {
            false
        }
    }

    pub fn get_received_data(&self) -> Vec<u8> {
        if let Ok(mut data) = self.received_data.lock() {
            let result = data.clone();
            data.clear();
            result
        } else {
            Vec::new()
        }
    }

    pub fn send_data(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_connected() {
            return Err("蓝牙未连接".into());
        }
        
        self.indicate(data)?;
        info!("通过BLE发送数据: {:?}", data);
        Ok(())
    }

    pub fn start_data_receiver(&self) {
        info!("BLE GATT服务器已启动，等待客户端连接...");
    }

    // 状态检查方法
    fn check_esp_status(&self, status: Result<(), EspError>) {
        if let Err(e) = status {
            warn!("收到ESP状态错误: {e:?}");
        }
    }

    fn check_bt_status(&self, status: BtStatus) -> Result<(), EspError> {
        match status {
            BtStatus::Success => Ok(()),
            BtStatus::Fail => {
                warn!("蓝牙操作失败");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            BtStatus::NotReady => {
                warn!("蓝牙未就绪");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            BtStatus::Busy => {
                warn!("蓝牙忙碌");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            _ => {
                warn!("收到蓝牙状态: {status:?}");
                // 对于其他状态，我们继续执行而不是失败
                Ok(())
            }
        }
    }

    fn check_gatt_status(&self, status: GattStatus) -> Result<(), EspError> {
        match status {
            GattStatus::Ok => Ok(()),
            GattStatus::Error => {
                warn!("GATT操作错误");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            GattStatus::InternalError => {
                warn!("GATT内部错误");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            GattStatus::Busy => {
                warn!("GATT忙碌");
                Err(EspError::from_infallible::<ESP_FAIL>())
            }
            _ => {
                warn!("收到GATT状态: {status:?}");
                // 对于其他状态，我们继续执行而不是失败
                Ok(())
            }
        }
    }
}

impl Clone for BluetoothManager {
    fn clone(&self) -> Self {
        Self {
            gap: self.gap.clone(),
            gatts: self.gatts.clone(),
            state: self.state.clone(),
            condvar: self.condvar.clone(),
            is_connected: self.is_connected.clone(),
            received_data: self.received_data.clone(),
        }
    }
}

impl Drop for BluetoothManager {
    fn drop(&mut self) {
        info!("正在关闭BLE连接...");
        info!("BLE连接已关闭");
    }
}
