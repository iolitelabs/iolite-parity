// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import BigNumber from 'bignumber.js';
import React, { Component, PropTypes } from 'react';
import { bindActionCreators } from 'redux';
import { connect } from 'react-redux';
import { observer } from 'mobx-react';

import Store from '../../store';
import * as RequestsActions from '../../../../redux/providers/signerActions';
import { Container, Page, TxList } from '../../../../ui';

import { RequestPending, RequestFinished } from '../../components';

import styles from './RequestsPage.css';

@observer
class RequestsPage extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired
  };

  static propTypes = {
    signer: PropTypes.shape({
      pending: PropTypes.array.isRequired,
      finished: PropTypes.array.isRequired
    }).isRequired,
    actions: PropTypes.shape({
      startConfirmRequest: PropTypes.func.isRequired,
      startRejectRequest: PropTypes.func.isRequired
    }).isRequired,
    isTest: PropTypes.bool.isRequired
  };

  store = new Store(this.context.api, true);

  componentWillUnmount () {
    this.store.unsubscribe();
  }

  render () {
    return (
      <Page>
        <div>{ this.renderPendingRequests() }</div>
        <div>{ this.renderLocalQueue() }</div>
        <div>{ this.renderFinishedRequests() }</div>
      </Page>
    );
  }

  _sortRequests = (a, b) => {
    return new BigNumber(b.id).cmp(a.id);
  }

  renderLocalQueue () {
    const { localHashes } = this.store;

    if (!localHashes.length) {
      return null;
    }

    return (
      <Container title='Local Transactions'>
        <TxList
          address=''
          hashes={ localHashes } />
      </Container>
    );
  }

  renderPendingRequests () {
    const { pending } = this.props.signer;

    if (!pending.length) {
      return (
        <Container>
          <div className={ styles.noRequestsMsg }>
            There are no requests requiring your confirmation.
          </div>
        </Container>
      );
    }

    const items = pending.sort(this._sortRequests).map(this.renderPending);

    return (
      <Container title='Pending Requests'>
        <div className={ styles.items }>
          { items }
        </div>
      </Container>
    );
  }

  renderFinishedRequests () {
    const { finished } = this.props.signer;

    if (!finished.length) {
      return;
    }

    const items = finished.sort(this._sortRequests).map(this.renderFinished);

    return (
      <Container title='Finished Requests'>
        <div className={ styles.items }>
          { items }
        </div>
      </Container>
    );
  }

  renderPending = (data) => {
    const { actions, isTest } = this.props;
    const { payload, id, isSending, date } = data;

    return (
      <RequestPending
        className={ styles.request }
        onConfirm={ actions.startConfirmRequest }
        onReject={ actions.startRejectRequest }
        isSending={ isSending || false }
        key={ id }
        id={ id }
        payload={ payload }
        date={ date }
        isTest={ isTest }
        store={ this.store }
      />
    );
  }

  renderFinished = (data) => {
    const { isTest } = this.props;
    const { payload, id, result, msg, status, error, date } = data;

    return (
      <RequestFinished
        className={ styles.request }
        result={ result }
        key={ id }
        id={ id }
        msg={ msg }
        status={ status }
        error={ error }
        payload={ payload }
        date={ date }
        isTest={ isTest }
        store={ this.store }
        />
    );
  }
}

function mapStateToProps (state) {
  const { isTest } = state.nodeStatus;
  const { actions, signer } = state;

  return {
    actions,
    signer,
    isTest
  };
}

function mapDispatchToProps (dispatch) {
  return {
    actions: bindActionCreators(RequestsActions, dispatch)
  };
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(RequestsPage);
